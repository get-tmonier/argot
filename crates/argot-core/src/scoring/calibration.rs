//! Calibration — port of `engine/argot/scoring/calibration/`.
//!
//! Collects sampleable hunks, calibrates a BPE threshold over multiple seeds,
//! builds the evidence corpus, and emits `scorer-config.json` (v2).
//!
//! Calibration-hunk sampling reproduces numpy's `default_rng(seed).choice(...)`
//! bit-for-bit (see [`crate::scoring::numpy_sampler`]), so the calibrated
//! `max(cal_scores)` threshold matches the Python engine exactly on every corpus.

use crate::bpe::BpeTokenizer;
use crate::scoring::adapters::python::PythonAdapter;
use crate::scoring::adapters::typescript::TypeScriptAdapter;
use crate::scoring::adapters::{Language, LanguageAdapter};
use crate::scoring::bpe_scorer::BpeScorer;
use crate::scoring::call_receiver::CallReceiverScorer;
use crate::scoring::typicality::TypicalityModel;
use crate::text::{read_text_lossy, splitlines, splitlines_keepends};
use anyhow::{bail, Result};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

const MIN_BODY_LINES: usize = 5;
const CONFIG_VERSION: u32 = 2;

// Production call-receiver constants (match calibration defaults).
const CR_ALPHA: f64 = 2.0;
const CR_CAP: usize = 5;
const CR_ROOT_BONUS: f64 = 2.0;
const CR_N_CLUSTERS: usize = 8;
const CR_CLUSTER_SEED: u64 = 0;
const CR_CLUSTER_BONUS: f64 = 5.0;

const EXCLUDE_DIRS: &[&str] = &[
    "test",
    "tests",
    "doc",
    "docs",
    "examples",
    "example",
    "migrations",
    "migration",
    "benchmarks",
    "benchmark",
    "fixtures",
    "scripts",
    "build",
    "dist",
    "__pycache__",
    ".git",
    ".history",
    ".tox",
    ".eggs",
];

fn basename(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Port of `is_excluded_path`. Public because the benchmark harness applies
/// the same calibration-scope filter to real-PR control hunks (lock-step:
/// calibration scope and scoring scope must agree).
pub fn is_excluded_path(path: &Path, source_dir: &Path) -> bool {
    let rel = match path.strip_prefix(source_dir) {
        Ok(r) => r,
        Err(_) => return true,
    };
    let comps: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if comps.is_empty() {
        return true;
    }
    for part in &comps[..comps.len() - 1] {
        if EXCLUDE_DIRS.contains(&part.as_str()) || part.starts_with("test") || part == "__tests__"
        {
            return true;
        }
    }
    let name = &comps[comps.len() - 1];
    if name.starts_with("test_") || name == "conftest.py" {
        return true;
    }
    if name.contains(".test.") || name.contains(".spec.") {
        return true;
    }
    if name.contains(".config.") {
        return true;
    }
    name.starts_with('.') && name[1..].contains("rc.")
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
pub struct Candidate {
    pub hunk: String,
    pub file_path: PathBuf,
    pub file_source: String,
}

/// Port of `collect_candidates_with_metadata` (exclude_data_dominant=True,
/// exclude_atypical=False).
pub fn collect_candidates(source_dir: &Path, adapter: &dyn LanguageAdapter) -> Vec<Candidate> {
    let exts: &[&str] = match adapter.language() {
        Language::Python => &[".py"],
        Language::Typescript => &[".ts", ".tsx"],
    };
    let mut out = Vec::new();
    for ext in exts {
        for src_file in rglob_sorted(source_dir, ext) {
            if is_excluded_path(&src_file, source_dir) {
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
    import_modules: Vec<String>,
    import_module_prefixes: Vec<String>,
    calibration: CalibrationMeta,
    evidence_corpus: EvidenceCorpusJson,
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
    }
}

/// Canonical config-key name for a scoring language ("python"/"typescript").
/// Public so `inspect` reports under the same keys `scorer-config.json` uses.
pub fn language_name(language: Language) -> &'static str {
    match language {
        Language::Python => "python",
        Language::Typescript => "typescript",
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
            let contrib = call_receiver.weighted_contribution_for_file(
                &c.hunk,
                Some(&c.file_path),
                0.0,
                0.0,
                cfg.cluster_bonus,
                cfg.cap,
                Some(&c.file_source),
            );
            cal_scores.push(raw_bpe + contrib);
        }
        // threshold_percentile default 100 → max.
        let t = cal_scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        seed_thresholds.push(if t.is_finite() { t } else { 0.0 });
    }
    seed_thresholds
}

/// Options for `run_calibrate` (defaults mirror the Python CLI).
pub struct CalibrateOptions {
    pub n_cal: usize,
    pub seed: u64,
    pub n_seeds: usize,
    pub evidence_top_n: usize,
    pub repo_sha: String,
    pub timestamp_utc: String,
}

impl Default for CalibrateOptions {
    fn default() -> Self {
        Self {
            n_cal: 500,
            seed: 0,
            n_seeds: 7,
            evidence_top_n: 50,
            repo_sha: "unknown".to_string(),
            timestamp_utc: String::new(),
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
        // import_modules = sorted(union of extract_imports over corpus).
        // calibrate builds the scorer with repo_root=None, so no
        // resolve_repo_modules (exact/prefix) contribution; prefixes stay empty.
        let mut repo_modules: HashSet<String> = HashSet::new();
        for s in &sources {
            repo_modules.extend(adapter.extract_imports(s));
        }
        let mut import_modules: Vec<String> = repo_modules.into_iter().collect();
        import_modules.sort();
        let import_module_prefixes: Vec<String> = Vec::new();
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
        let candidates = collect_candidates(repo_dir, adapter.as_ref());
        let effective_n_cal = opts.n_cal.min(candidates.len());
        let typicality = TypicalityModel::new(language);

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
                call_receiver_cluster_rare_threshold: 0,
                call_receiver_cluster_size_min: 0,
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
