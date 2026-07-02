//! Calibration — port of `engine/argot/scoring/calibration/`.
//!
//! Collects sampleable hunks, calibrates a BPE threshold over multiple seeds,
//! builds the evidence corpus, and emits `scorer-config.json` (v3, carrying
//! the fit-time model snapshot).
//!
//! Calibration-hunk sampling reproduces numpy's `default_rng(seed).choice(...)`
//! bit-for-bit (see [`crate::scoring::numpy_sampler`]), so the calibrated
//! `max(cal_scores)` threshold matches the Python engine exactly on every corpus.

use crate::bpe::BpeTokenizer;
use crate::scoring::adapters::python::PythonAdapter;
use crate::scoring::adapters::ruby::RubyAdapter;
use crate::scoring::adapters::typescript::TypeScriptAdapter;
use crate::scoring::adapters::{Language, LanguageAdapter};
use crate::scoring::bpe_scorer::BpeScorer;
use crate::scoring::call_receiver::CallReceiverScorer;
use crate::scoring::conventions::{fit_convention_frequencies, ConventionScorer};
use crate::scoring::model::LanguageModel;
use crate::scoring::typicality::TypicalityModel;
use crate::suppress::PathSuppressions;
use crate::text::{read_text_lossy, splitlines, splitlines_keepends};
use anyhow::{bail, Result};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

const MIN_BODY_LINES: usize = 5;
/// v3: adds the per-language `model` block (fit-time BPE stats + callee
/// attestation snapshot) and repo-owned import modules. Check refuses other
/// versions — regenerate via `argot fit`.
const CONFIG_VERSION: u32 = 3;

// Production call-receiver constants (match calibration defaults).
const CR_ALPHA: f64 = 2.0;
const CR_CAP: usize = 5;
const CR_ROOT_BONUS: f64 = 2.0;
const CR_N_CLUSTERS: usize = 8;
const CR_CLUSTER_SEED: u64 = 0;
const CR_CLUSTER_BONUS: f64 = 5.0;
/// Real diff hunks routinely start or end mid-construct (git picks hunk
/// boundaries, not the parser), so a bare-fragment parse error is the NORM
/// for check-time hunks, not an edge case — without the host fallback the
/// call-receiver contributes 0 on exactly the hunks check scores. The
/// calibration side has always applied the fallback (candidates carry their
/// file region), so enabling it at check time is what makes the two paths
/// symmetric. Era-14 gated it off based on catalog-mode FP with a forced
/// cluster-rare rule; in production the rare rule is auto-detected per
/// corpus, and the era-15 production-path FP controls re-validated it.
const CR_PARSE_ERROR_FALLBACK: bool = true;
/// Score added when a hunk's rarest present convention clears its calibrated
/// bar (era 15; same magnitude as the cluster bonus).
const CONVENTION_BONUS: f64 = 5.0;

fn basename(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The built-in `argot:recommended` path exclusion (formerly the hardcoded
/// list here; now [`crate::suppress::recommended_excluded`]). Public because
/// the benchmark harness applies the same calibration-scope filter to real-PR
/// control hunks — bench calls resolve to recommended-set-only semantics
/// (lock-step: calibration scope and scoring scope must agree).
pub fn is_excluded_path(path: &Path, source_dir: &Path) -> bool {
    match crate::suppress::rel_string(path, source_dir) {
        Some(rel) => crate::suppress::recommended_excluded(&rel),
        None => true,
    }
}

/// Recursively list files under `dir` matching `ext` (e.g. ".py"), sorted.
fn rglob_sorted(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    fn walk(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(t) if t.is_dir() => walk(&path, ext, out),
                Ok(t) if t.is_file() && basename(&path).ends_with(ext) => out.push(path),
                _ => {}
            }
        }
    }
    walk(dir, ext, &mut out);
    out.sort();
    out
}

/// A calibration candidate: hunk text + originating file path + file source.
/// Line bounds are 1-indexed inclusive within `file_source` and back the
/// era-14 phase D parse-error callee fallback.
pub struct Candidate {
    pub hunk: String,
    pub file_path: PathBuf,
    pub file_source: String,
    pub hunk_start_line: usize,
    pub hunk_end_line: usize,
}

/// Port of `collect_candidates_with_metadata` (exclude_data_dominant=True,
/// exclude_atypical=False), against the built-in recommended path set only —
/// existing callers (the benchmark harness) keep today's behaviour exactly.
/// The production calibrator resolves `.argotignore` on top via
/// [`collect_candidates_with`].
pub fn collect_candidates(source_dir: &Path, adapter: &dyn LanguageAdapter) -> Vec<Candidate> {
    collect_candidates_with(source_dir, adapter, &PathSuppressions::recommended())
}

/// [`collect_candidates`] against a fully resolved path-suppression set
/// (recommended built-ins + `.argotignore`). Calibration sampling, the
/// check-time scope filter, and `argot inspect` all consult the same
/// [`PathSuppressions`] so their scopes stay in lock-step.
pub fn collect_candidates_with(
    source_dir: &Path,
    adapter: &dyn LanguageAdapter,
    path_suppressions: &PathSuppressions,
) -> Vec<Candidate> {
    let exts: &[&str] = match adapter.language() {
        Language::Python => &[".py"],
        Language::Typescript => &[".ts", ".tsx"],
        Language::Ruby => &[".rb"],
    };
    let mut out = Vec::new();
    for ext in exts {
        for src_file in rglob_sorted(source_dir, ext) {
            if path_suppressions.is_suppressed_abs(&src_file, source_dir) {
                continue;
            }
            let source = match read_text_lossy(&src_file) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if adapter.is_data_dominant(&source) {
                continue;
            }
            let lines = splitlines(&source);
            for (start, end) in adapter.enumerate_sampleable_ranges(&source) {
                if end.saturating_sub(start) < MIN_BODY_LINES {
                    continue;
                }
                // hunk_text = "\n".join(lines[start-1:end]) (1-indexed inclusive)
                let s = start.saturating_sub(1);
                let e = end.min(lines.len());
                if s >= e {
                    continue;
                }
                let hunk = lines[s..e].join("\n");
                out.push(Candidate {
                    hunk,
                    file_path: src_file.clone(),
                    file_source: source.clone(),
                    hunk_start_line: s + 1,
                    hunk_end_line: e,
                });
            }
        }
    }
    out
}

/// Sample of `n` distinct indices from `[0, len)`, returned sorted ascending —
/// bit-exact reproduction of Python's
/// `sorted(np.random.default_rng(seed).choice(len, n, replace=False))`
/// (see [`crate::scoring::numpy_sampler`]). Matching numpy's RNG here keeps the
/// calibrated `max(cal_scores)` threshold identical to the Python engine.
/// Public so the benchmark harness samples with the same RNG.
pub fn sample_indices(len: usize, n: usize, seed: u64) -> Vec<usize> {
    crate::scoring::numpy_sampler::choice_sorted(len, n, seed)
}

/// Reimpl of `_blank_prose_lines` (keepends).
fn blank_prose_lines(src: &str, ranges: &HashSet<usize>) -> String {
    if ranges.is_empty() {
        return src.to_string();
    }
    let lines = splitlines_keepends(src);
    let mut result = String::with_capacity(src.len());
    for (i, line) in lines.iter().enumerate() {
        if ranges.contains(&(i + 1)) {
            if line.ends_with('\n') {
                result.push('\n');
            }
        } else {
            result.push_str(line);
        }
    }
    result
}

// --- scorer-config.json serialisation shapes ---

#[derive(Serialize)]
struct CommonEntry {
    name: String,
    count: usize,
}

#[derive(Serialize)]
struct Totals {
    import_specifiers_attested: usize,
    callees_attested_by_cluster: BTreeMap<String, usize>,
}

#[derive(Serialize)]
struct EvidenceCorpusJson {
    imports: Vec<CommonEntry>,
    identifiers: BTreeMap<String, usize>,
    callees_by_cluster: BTreeMap<String, Vec<CommonEntry>>,
    totals: Totals,
}

#[derive(Serialize)]
struct CalibrationMeta {
    n_cal: usize,
    seed: u64,
    n_seeds: usize,
    repo_sha: String,
    timestamp_utc: String,
}

#[derive(Serialize)]
struct LangConfig {
    threshold: f64,
    call_receiver_alpha: f64,
    call_receiver_cap: usize,
    call_receiver_root_bonus: f64,
    call_receiver_n_clusters: usize,
    call_receiver_cluster_seed: u64,
    call_receiver_cluster_bonus: f64,
    call_receiver_cluster_rare_threshold: usize,
    call_receiver_cluster_size_min: usize,
    call_receiver_parse_error_host_fallback: bool,
    convention_bonus: f64,
    import_modules: Vec<String>,
    import_module_prefixes: Vec<String>,
    calibration: CalibrationMeta,
    evidence_corpus: EvidenceCorpusJson,
    /// Fingerprint of `model` (deterministic serialization → stable hash).
    model_hash: String,
    /// Fit-time model snapshot: BPE token stats + callee attestation +
    /// cluster partition. Check scores against this, never the live tree.
    model: LanguageModel,
}

#[derive(Serialize)]
struct ScorerConfig {
    version: u32,
    languages: BTreeMap<String, LangConfig>,
}

fn adapter_for(language: Language) -> Box<dyn LanguageAdapter> {
    match language {
        Language::Python => Box::new(PythonAdapter::new()),
        Language::Typescript => Box::new(TypeScriptAdapter::new()),
        Language::Ruby => Box::new(RubyAdapter::new()),
    }
}

/// Canonical config-key name for a scoring language ("python"/"typescript").
/// Public so `inspect` reports under the same keys `scorer-config.json` uses.
pub fn language_name(language: Language) -> &'static str {
    match language {
        Language::Python => "python",
        Language::Typescript => "typescript",
        Language::Ruby => "ruby",
    }
}

/// Extension → language routing used to partition the corpus (`.py` → python;
/// `.ts`/`.tsx`/`.js`/`.jsx` → typescript). Public so `inspect` classifies
/// files with exactly the calibration routing.
pub fn language_for_filename(name: &str) -> Option<Language> {
    let ext = match name.rfind('.') {
        Some(i) => &name[i..],
        None => return None,
    };
    match ext {
        ".py" => Some(Language::Python),
        ".ts" | ".tsx" | ".js" | ".jsx" => Some(Language::Typescript),
        ".rb" => Some(Language::Ruby),
        _ => None,
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// Knobs for [`multi_seed_thresholds`].
pub struct ThresholdRunConfig {
    /// Calibration hunks sampled per seed (callers pre-clamp to the candidate
    /// count).
    pub n_cal: usize,
    /// First seed; seeds run `base_seed .. base_seed + n_seeds`.
    pub base_seed: u64,
    pub n_seeds: usize,
    /// Cluster-bonus magnitude applied to calibration-side contributions.
    pub cluster_bonus: f64,
    /// Cap on unattested callees counted per hunk.
    pub cap: f64,
}

/// Per-seed calibration thresholds: for each seed, `max` over sampled
/// cal-hunk scores (BPE + cluster contribution at alpha/root_bonus 0).
///
/// Shared by the production calibrator ([`run_calibrate`] takes the median)
/// and the benchmark harness (which also reports threshold CV across seeds) so
/// both stay in lock-step. Whether optional contributions (cluster-rare,
/// shape primitives) apply on the calibration side is decided by how the
/// caller constructs `call_receiver` — the asymmetric-calibration default
/// builds it with `cluster_rare_threshold = 0`.
pub fn multi_seed_thresholds(
    candidates: &[Candidate],
    bpe: &BpeScorer,
    call_receiver: &mut CallReceiverScorer,
    adapter: &dyn LanguageAdapter,
    typicality: &TypicalityModel,
    cfg: &ThresholdRunConfig,
) -> Vec<f64> {
    let effective_n_cal = cfg.n_cal.min(candidates.len());
    let mut seed_thresholds = Vec::new();
    for k in 0..cfg.n_seeds {
        let seed = cfg.base_seed.wrapping_add(k as u64);
        let idx = sample_indices(candidates.len(), effective_n_cal, seed);
        let mut cal_scores = Vec::new();
        for &i in &idx {
            let c = &candidates[i];
            if typicality.is_atypical(&c.hunk).0 {
                continue;
            }
            let prose = adapter.prose_line_ranges(&c.hunk);
            let raw_bpe = bpe.bpe_score(&blank_prose_lines(&c.hunk, &prose));
            // Cal side scores without local-binding attestation: candidates
            // are corpus files whose callees are attested anyway, so the
            // omission only leaves the threshold marginally conservative.
            let contrib = call_receiver.weighted_contribution_for_file(
                &c.hunk,
                Some(&c.file_path),
                0.0,
                0.0,
                cfg.cluster_bonus,
                cfg.cap,
                Some(&c.file_source),
                Some((&c.file_source, c.hunk_start_line, c.hunk_end_line)),
                &Default::default(),
            );
            cal_scores.push(raw_bpe + contrib);
        }
        // threshold_percentile default 100 → max.
        let t = cal_scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        seed_thresholds.push(if t.is_finite() { t } else { 0.0 });
    }
    seed_thresholds
}

/// Options for `run_calibrate` (defaults mirror the Python CLI, including the
/// era-13.5 asymmetric-calibration knobs the final Python calibrator shipped).
pub struct CalibrateOptions {
    pub n_cal: usize,
    pub seed: u64,
    pub n_seeds: usize,
    pub evidence_top_n: usize,
    pub repo_sha: String,
    pub timestamp_utc: String,
    /// Cluster-rare threshold for the CHECK-time scorer: a callee attested in
    /// ≤ N cluster files is treated as cluster-absent. 0 disables the rule
    /// (pre-13.5 baseline). Calibration itself always runs with the rule off
    /// (asymmetric calibration — see docs/agents/calibration-contract.md).
    pub cluster_rare_threshold: usize,
    /// Minimum cluster size for the rare rule to fire.
    pub cluster_size_min: usize,
    /// Per-corpus auto-detect: probe the calibration distribution's rare-rule
    /// fire rate; keep the rule when it is discriminative (fire rate below
    /// `asym_fire_rate_threshold`), disable it when noisy (would FP-flood).
    pub auto_select_asym_cal: bool,
    pub asym_fire_rate_threshold: f64,
}

impl Default for CalibrateOptions {
    fn default() -> Self {
        Self {
            // n_cal=100 × 7 seeds is the configuration every era's bench
            // validated the recall/FP gates against. The production default
            // previously sampled 500, which systematically raises the
            // max-of-sample threshold above the validated one (measured:
            // saleor 5.97 vs 5.44, hono 5.87 vs 4.27) and cost real recall
            // through the production path.
            n_cal: 100,
            seed: 0,
            n_seeds: 7,
            evidence_top_n: 50,
            repo_sha: "unknown".to_string(),
            timestamp_utc: String::new(),
            cluster_rare_threshold: 2,
            cluster_size_min: 0,
            auto_select_asym_cal: true,
            asym_fire_rate_threshold: 0.05,
        }
    }
}

/// Run calibration and write `scorer-config.json` to `output`.
///
/// `repo_dir` is the target repo (candidate rglob source). `repo_corpus_path`
/// lists corpus files (from `train`). `generic_baseline_json` is the embedded
/// baseline bytes.
pub fn run_calibrate(
    repo_dir: &Path,
    repo_corpus_path: &Path,
    generic_baseline_json: &[u8],
    output: &Path,
    opts: &CalibrateOptions,
) -> Result<Vec<(String, f64)>> {
    // Canonicalize so candidate paths (rglobbed from here) share the corpus
    // paths' prefix — cluster routing at calibration time must resolve by
    // path exactly as check-time routing resolves against the model's
    // repo-relative keys.
    let repo_dir = &std::fs::canonicalize(repo_dir).unwrap_or_else(|_| repo_dir.to_path_buf());
    let corpus_txt = read_text_lossy(repo_corpus_path)
        .map_err(|_| anyhow::anyhow!("repo corpus not found: {}", repo_corpus_path.display()))?;
    let corpus_files: Vec<PathBuf> = corpus_txt
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(PathBuf::from)
        .collect();
    if corpus_files.is_empty() {
        bail!("empty repo corpus");
    }

    // Partition corpus by language.
    let mut by_lang: BTreeMap<&'static str, (Language, Vec<PathBuf>)> = BTreeMap::new();
    for f in &corpus_files {
        if let Some(lang) = language_for_filename(&basename(f)) {
            by_lang
                .entry(language_name(lang))
                .or_insert_with(|| (lang, Vec::new()))
                .1
                .push(f.clone());
        }
    }
    if by_lang.is_empty() {
        bail!("no recognized language files in repo corpus");
    }

    let mut languages: BTreeMap<String, LangConfig> = BTreeMap::new();
    let mut thresholds_out: Vec<(String, f64)> = Vec::new();

    // Resolved path-suppression set (recommended built-ins + `.argotignore`) —
    // the same set `check` filters against (lock-step principle).
    let path_suppressions = PathSuppressions::load(repo_dir);

    for (name, (language, lang_files)) in by_lang {
        let adapter = adapter_for(language);

        // Read corpus sources once (shared by BPE + call-receiver + evidence).
        let repo_files: Vec<(PathBuf, String)> = lang_files
            .iter()
            .filter_map(|p| read_text_lossy(p).ok().map(|s| (p.clone(), s)))
            .collect();
        // exclude_data_dominant filter.
        let filtered: Vec<(PathBuf, String)> = repo_files
            .iter()
            .filter(|(_, s)| !adapter.is_data_dominant(s))
            .cloned()
            .collect();
        let corpus = if filtered.is_empty() {
            &repo_files
        } else {
            &filtered
        };
        let sources: Vec<String> = corpus.iter().map(|(_, s)| s.clone()).collect();

        let bpe = BpeScorer::new(BpeTokenizer::load(), generic_baseline_json, &sources)?;
        // import_modules = corpus imports + repo-owned module names
        // (package/tsconfig aliases). Folding resolve_repo_modules matches
        // the bench scorer's import surface: a repo-internal module the
        // corpus never happened to import is still not a foreign voice.
        let mut modules: HashSet<String> = HashSet::new();
        for s in &sources {
            modules.extend(adapter.extract_imports(s));
        }
        let resolved = adapter.resolve_repo_modules(repo_dir);
        modules.extend(resolved.exact.iter().cloned());
        let mut import_modules: Vec<String> = modules.into_iter().collect();
        import_modules.sort();
        let mut import_module_prefixes: Vec<String> = resolved.prefixes.into_iter().collect();
        import_module_prefixes.sort();
        let mut call_receiver = CallReceiverScorer::new(
            corpus,
            language,
            CR_ALPHA,
            CR_CAP,
            adapter.as_ref(),
            CR_N_CLUSTERS,
            CR_CLUSTER_SEED,
            0,
            0,
        )
        .map_err(anyhow::Error::msg)?;

        // Candidates for sampling.
        let candidates = collect_candidates_with(repo_dir, adapter.as_ref(), &path_suppressions);
        let effective_n_cal = opts.n_cal.min(candidates.len());
        let typicality = TypicalityModel::new(language);

        // Era-13.5 per-corpus auto-detect: probe the rare rule's fire rate on
        // sampled calibration hunks; a rule that fires often on ordinary code
        // would FP-flood at check time, so fall back to baseline (rare=0).
        let mut resolved_rare = opts.cluster_rare_threshold;
        if opts.auto_select_asym_cal
            && resolved_rare > 0
            && CR_N_CLUSTERS > 1
            && !candidates.is_empty()
        {
            let mut probe_cr = CallReceiverScorer::new(
                corpus,
                language,
                CR_ALPHA,
                CR_CAP,
                adapter.as_ref(),
                CR_N_CLUSTERS,
                CR_CLUSTER_SEED,
                resolved_rare,
                opts.cluster_size_min,
            )
            .map_err(anyhow::Error::msg)?;
            let idx = sample_indices(candidates.len(), effective_n_cal, opts.seed);
            let mut hunks_scored = 0usize;
            for &i in &idx {
                let c = &candidates[i];
                if typicality.is_atypical(&c.hunk).0 {
                    continue;
                }
                probe_cr.weighted_contribution_for_file(
                    &c.hunk,
                    Some(&c.file_path),
                    0.0,
                    0.0,
                    CR_CLUSTER_BONUS,
                    CR_CAP as f64,
                    Some(&c.file_source),
                    Some((&c.file_source, c.hunk_start_line, c.hunk_end_line)),
                    &Default::default(),
                );
                hunks_scored += 1;
            }
            let fire_rate = probe_cr.rare_branch_hunks_fired as f64 / hunks_scored.max(1) as f64;
            let keep_rule = fire_rate < opts.asym_fire_rate_threshold;
            eprintln!(
                "[{name}][auto-asym] cluster_rare probe: rare_hunks_fired={}/{} fire_rate={:.3} threshold={:.3} → {}",
                probe_cr.rare_branch_hunks_fired,
                hunks_scored,
                fire_rate,
                opts.asym_fire_rate_threshold,
                if keep_rule {
                    "KEEP rule"
                } else {
                    "DISABLE rule (rare=0)"
                }
            );
            if !keep_rule {
                resolved_rare = 0;
            }
        }

        let seed_thresholds = multi_seed_thresholds(
            &candidates,
            &bpe,
            &mut call_receiver,
            adapter.as_ref(),
            &typicality,
            &ThresholdRunConfig {
                n_cal: effective_n_cal,
                base_seed: opts.seed,
                n_seeds: opts.n_seeds,
                cluster_bonus: CR_CLUSTER_BONUS,
                cap: CR_CAP as f64,
            },
        );
        let threshold = median(seed_thresholds);

        // Evidence corpus.
        let evidence = build_evidence_corpus(
            &lang_files,
            adapter.as_ref(),
            &call_receiver,
            opts.evidence_top_n,
        );

        // Convention-rarity model: corpus frequencies plus firing bars set at
        // the max feature value over the same multi-seed calibration sample
        // the threshold uses — the stage stays silent on in-voice code, and
        // per the calibration contract it never feeds the threshold itself.
        let mut convention_model = fit_convention_frequencies(corpus, language);
        {
            // Bars over ALL candidates (not the threshold's n_cal sample):
            // the bar is a max-gate, so sampling only adds noise — a smaller
            // sample lowers the bar and fires the stage on ordinary code.
            // Over the full candidate population the bar is deterministic
            // and maximally conservative: a convention fires only when rarer
            // than anything the repo's own sampleable code contains.
            let conv = ConventionScorer::new(convention_model.clone(), language);
            let mut syntax_bar = 0.0f64;
            let mut ident_bar = 0.0f64;
            for c in &candidates {
                if typicality.is_atypical(&c.hunk).0 {
                    continue;
                }
                let scores = conv.scores(
                    &c.hunk,
                    Some((&c.file_source, c.hunk_start_line, c.hunk_end_line)),
                );
                syntax_bar = syntax_bar.max(scores.syntax_surprisal);
                ident_bar = ident_bar.max(scores.ident_surprisal);
            }
            convention_model.syntax_bar = syntax_bar;
            convention_model.ident_bar = ident_bar;
        }

        // Fit-time model snapshot: the calibration call-receiver's fitted
        // state is threshold-parameter-independent (rare/alpha are scoring
        // knobs, not fitted state), so exporting from it is exact.
        let model = LanguageModel {
            bpe: bpe.stats(),
            call_receiver: call_receiver.export_model(repo_dir),
            conventions: Some(convention_model),
        };
        let model_hash = model.hash();

        thresholds_out.push((name.to_string(), threshold));
        languages.insert(
            name.to_string(),
            LangConfig {
                threshold,
                call_receiver_alpha: CR_ALPHA,
                call_receiver_cap: CR_CAP,
                call_receiver_root_bonus: CR_ROOT_BONUS,
                call_receiver_n_clusters: CR_N_CLUSTERS,
                call_receiver_cluster_seed: CR_CLUSTER_SEED,
                call_receiver_cluster_bonus: CR_CLUSTER_BONUS,
                call_receiver_cluster_rare_threshold: resolved_rare,
                call_receiver_cluster_size_min: opts.cluster_size_min,
                call_receiver_parse_error_host_fallback: CR_PARSE_ERROR_FALLBACK,
                convention_bonus: CONVENTION_BONUS,
                import_modules,
                import_module_prefixes,
                calibration: CalibrationMeta {
                    n_cal: effective_n_cal,
                    seed: opts.seed,
                    n_seeds: opts.n_seeds,
                    repo_sha: opts.repo_sha.clone(),
                    timestamp_utc: opts.timestamp_utc.clone(),
                },
                evidence_corpus: evidence,
                model_hash,
                model,
            },
        );
    }

    let config = ScorerConfig {
        version: CONFIG_VERSION,
        languages,
    };
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let json = serde_json::to_string_pretty(&config)?;
    std::fs::write(output, json)?;
    Ok(thresholds_out)
}

/// Build the evidence corpus (`build_evidence_corpus`).
fn build_evidence_corpus(
    files: &[PathBuf],
    adapter: &dyn LanguageAdapter,
    call_receiver: &CallReceiverScorer,
    top_n: usize,
) -> EvidenceCorpusJson {
    use std::collections::HashMap;
    // imports: per-file distinct specifiers.
    let mut import_counts: HashMap<String, usize> = HashMap::new();
    let mut identifier_counts: HashMap<String, usize> = HashMap::new();
    let noise = adapter.identifier_noise();
    for path in files {
        let source = match read_text_lossy(path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for spec in adapter.extract_imports(&source) {
            *import_counts.entry(spec).or_insert(0) += 1;
        }
        let prose = adapter.prose_line_ranges(&source);
        let clean = if prose.is_empty() {
            source.clone()
        } else {
            blank_prose_lines(&source, &prose)
        };
        for ident in extract_identifiers(&clean) {
            if !noise.contains(&ident) {
                *identifier_counts.entry(ident).or_insert(0) += 1;
            }
        }
    }

    let callees_by_cluster = call_receiver.cluster_callee_counts_for_evidence();
    let mut cbc: BTreeMap<String, Vec<CommonEntry>> = BTreeMap::new();
    let mut attested: BTreeMap<String, usize> = BTreeMap::new();
    for (cid, counts) in callees_by_cluster {
        cbc.insert(cid.to_string(), top_n_entries(counts, top_n));
        attested.insert(cid.to_string(), counts.len());
    }

    EvidenceCorpusJson {
        imports: top_n_entries(&import_counts, top_n),
        identifiers: identifier_counts.into_iter().collect(),
        callees_by_cluster: cbc,
        totals: Totals {
            import_specifiers_attested: import_counts.len(),
            callees_attested_by_cluster: attested,
        },
    }
}

fn top_n_entries(counts: &std::collections::HashMap<String, usize>, n: usize) -> Vec<CommonEntry> {
    let mut items: Vec<(&String, &usize)> = counts.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    items
        .into_iter()
        .take(n)
        .map(|(name, count)| CommonEntry {
            name: name.clone(),
            count: *count,
        })
        .collect()
}

/// `_IDENTIFIER_RE = \b[A-Za-z_][A-Za-z0-9_]*\b`.
fn extract_identifiers(src: &str) -> Vec<String> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        let is_start = c == b'_' || c.is_ascii_alphabetic();
        if is_start {
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i] == b'_' || bytes[i].is_ascii_alphanumeric()) {
                i += 1;
            }
            out.push(src[start..i].to_string());
        } else {
            i += 1;
        }
    }
    out
}
