//! Bench scorer construction — the Rust equivalent of the retired Python
//! harness's `argot_bench.score.build_scorer`.
//!
//! Builds a calibrated `SequentialImportBpeScorer` for one (corpus, language):
//! per-corpus auto-detect probe for the cluster-rare rule, multi-seed median
//! threshold via `argot_core::scoring::calibration::multi_seed_thresholds`
//! (the same code path the production calibrator uses), then a check-time
//! scorer with the full parameter set.

use anyhow::{bail, Context, Result};
use argot_core::scoring::adapters::go::GoAdapter;
use argot_core::scoring::adapters::c::CAdapter;
use argot_core::scoring::adapters::java::JavaAdapter;
use argot_core::scoring::adapters::csharp::CSharpAdapter;
use argot_core::scoring::adapters::php::PhpAdapter;
use argot_core::scoring::adapters::python::PythonAdapter;
use argot_core::scoring::adapters::rust::RustAdapter;
use argot_core::scoring::adapters::typescript::TypeScriptAdapter;
use argot_core::scoring::adapters::{Language, LanguageAdapter};
use argot_core::scoring::bpe_scorer::BpeScorer;
use argot_core::scoring::calibration::{
    collect_candidates, is_excluded_path, multi_seed_thresholds, sample_indices, Candidate,
    ThresholdRunConfig,
};
use argot_core::scoring::call_receiver::{CallReceiverScorer, RarityWeighting};
use argot_core::scoring::conventions::{fit_convention_frequencies, ConventionScorer};
use argot_core::scoring::sequential::{SequentialConfig, SequentialImportBpeScorer};
use argot_core::scoring::typicality::TypicalityModel;
use argot_core::text::read_text_lossy;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// All scorer knobs, defaulting to the era-13.5 canonical bench config.
#[derive(Debug, Clone)]
pub struct BenchKnobs {
    pub n_cal: usize,
    pub seed: u64,
    pub threshold_n_seeds: usize,
    pub alpha: f64,
    pub cap: usize,
    pub root_bonus: f64,
    pub n_clusters: usize,
    pub cluster_seed: u64,
    pub cluster_bonus: f64,
    pub cluster_rare_threshold: usize,
    pub cluster_size_min: usize,
    pub enable_typicality_filter: bool,
    /// Symmetric-calibration mode: apply optional contributions (cluster-rare)
    /// on the calibration side too. Default false = asymmetric calibration.
    pub apply_optional_contributions_to_cal: bool,
    pub auto_select_asym_cal: bool,
    pub asym_fire_rate_threshold: f64,
    pub asym_probe_n: usize,
    /// Era-14 phase A: rarity weighting on the cluster branches, applied
    /// symmetrically to the probe, calibration, and scoring call-receivers.
    pub rarity_weighting: RarityWeighting,
    /// Era-14 phase B: calibration-hunk distribution — random source-file
    /// hunks (era-13.5 default) or real diff hunks from the extract dataset.
    pub calibration_source: CalibrationSource,
    /// Era-14 phase C: shape primitives enabled on the scoring path (empty =
    /// none). Asymmetric calibration: never applied to the cal side.
    pub shape_primitive_names: Vec<String>,
    /// Era-14 phase D: parse-error host fallback on the scoring path. On by
    /// default since era 15 (production check runs with it on; git-shaped
    /// fragments need it) — `--no-parse-error-fallback` restores the era-14
    /// catalog baseline.
    pub parse_error_host_fallback: bool,
    /// Era-15 convention-rarity stage (corpus-frequency model + calibrated
    /// bars). On by default to match production; `--no-conventions` restores
    /// the pre-era-15 catalog baseline.
    pub enable_conventions: bool,
    pub convention_bonus: f64,
}

/// Where calibration hunks come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationSource {
    /// Random sampleable source-file ranges (era-13.5 production behaviour).
    Random,
    /// Diff hunks from `dataset.jsonl` — structurally what real-PR controls
    /// look like, so the threshold is tighter and more honest. Scope filters
    /// stay in lock-step with control scoring: excluded paths are dropped and
    /// hunks are filtered to the scorer's language.
    Diff,
}

impl Default for BenchKnobs {
    fn default() -> Self {
        Self {
            n_cal: 100,
            seed: 0,
            threshold_n_seeds: 7,
            alpha: 2.0,
            cap: 5,
            root_bonus: 2.0,
            n_clusters: 8,
            cluster_seed: 0,
            cluster_bonus: 5.0,
            cluster_rare_threshold: 2,
            cluster_size_min: 0,
            enable_typicality_filter: true,
            apply_optional_contributions_to_cal: false,
            auto_select_asym_cal: true,
            asym_fire_rate_threshold: 0.05,
            asym_probe_n: 1000,
            rarity_weighting: RarityWeighting::Off,
            calibration_source: CalibrationSource::Random,
            shape_primitive_names: Vec::new(),
            parse_error_host_fallback: true,
            enable_conventions: true,
            convention_bonus: 5.0,
        }
    }
}

/// A calibrated scorer plus the calibration observables the report needs.
pub struct BenchScorer {
    pub scorer: SequentialImportBpeScorer,
    /// Per-seed thresholds from multi-seed calibration (median is the active
    /// threshold).
    pub seed_thresholds: Vec<f64>,
    pub threshold: f64,
    /// Cluster-rare threshold after the auto-detect probe (0 = rule disabled
    /// for this corpus).
    pub resolved_rare_threshold: usize,
}

pub fn adapter_for(language: Language) -> Box<dyn LanguageAdapter> {
    match language {
        Language::Python => Box::new(PythonAdapter::new()),
        Language::Typescript => Box::new(TypeScriptAdapter::new()),
        Language::Go => Box::new(GoAdapter::new()),
        Language::Rust => Box::new(RustAdapter::new()),
        Language::C => Box::new(CAdapter::new()),
        Language::Java => Box::new(JavaAdapter::new()),
        Language::CSharp => Box::new(CSharpAdapter::new()),
        Language::Php => Box::new(PhpAdapter::new()),
    }
}

pub fn parse_language(name: &str) -> Result<Language> {
    match name {
        "python" => Ok(Language::Python),
        "typescript" => Ok(Language::Typescript),
        "go" => Ok(Language::Go),
        "rust" => Ok(Language::Rust),
        "c" => Ok(Language::C),
        "java" => Ok(Language::Java),
        "csharp" => Ok(Language::CSharp),
        "php" => Ok(Language::Php),
        other => bail!("unsupported language {other:?}"),
    }
}

/// Corpus file list in the retired harness's order: for each extension in
/// sorted order, all matching files sorted. Cluster assignment is sensitive to
/// corpus order, so this must match the Python `_source_files` exactly.
pub fn source_files(repo_dir: &Path, language: Language) -> Vec<PathBuf> {
    let exts: &[&str] = match language {
        Language::Python => &[".py"],
        Language::Typescript => &[".ts", ".tsx"],
        Language::Go => &[".go"],
        Language::Rust => &[".rs"],
        Language::C => &[".c", ".h"],
        Language::Java => &[".java"],
        Language::CSharp => &[".cs"],
        Language::Php => &[".php"],
    };
    let mut out = Vec::new();
    for ext in exts {
        let mut batch = Vec::new();
        walk_files(repo_dir, &mut |p| {
            if p.file_name()
                .map(|n| n.to_string_lossy().ends_with(ext))
                .unwrap_or(false)
            {
                batch.push(p.to_path_buf());
            }
        });
        batch.sort();
        out.extend(batch);
    }
    out
}

fn walk_files(dir: &Path, visit: &mut dyn FnMut(&Path)) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(t) if t.is_dir() => walk_files(&path, visit),
            Ok(t) if t.is_file() => visit(&path),
            _ => {}
        }
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

/// `git show <sha>:<path>` in `repo_dir`, or `None` on any failure.
fn git_show_file(repo_dir: &Path, commit_sha: &str, file_path: &str) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .arg("show")
        .arg(format!("{commit_sha}:{file_path}"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Sample up to `n` diff hunks from `dataset.jsonl` for the auto-detect probe
/// (also used by research scouts as a real-PR hunk sample).
/// Returns `(hunk_content, file_abs_path, file_source)` tuples. File content is
/// read at the extraction commit via `git show` so line bounds never go stale.
///
/// The retired harness reservoir-sampled with Python's `random.Random`; the
/// selection here uses the numpy-exact sampler instead. The probe only feeds a
/// binary keep/disable decision on the fire *rate*, which is insensitive to
/// which particular hunks are drawn.
pub fn load_diff_hunks_for_probe(
    dataset_path: &Path,
    repo_dir: &Path,
    n: usize,
    seed: u64,
) -> Vec<(String, PathBuf, String)> {
    #[derive(serde::Deserialize)]
    struct Rec {
        file_path: String,
        hunk_start_line: usize,
        hunk_end_line: usize,
        commit_sha: String,
    }

    let raw = match std::fs::read(dataset_path) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut quads: Vec<(String, usize, usize, String)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for line in raw.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let rec: Rec = match serde_json::from_slice(line) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let key = format!(
            "{}:{}:{}:{}",
            rec.commit_sha, rec.file_path, rec.hunk_start_line, rec.hunk_end_line
        );
        if seen.insert(key) {
            quads.push((
                rec.file_path,
                rec.hunk_start_line,
                rec.hunk_end_line,
                rec.commit_sha,
            ));
        }
    }
    let take = n.min(quads.len());
    let idx = sample_indices(quads.len(), take, seed);

    let mut file_cache: HashMap<(String, String), Option<String>> = HashMap::new();
    let mut out = Vec::new();
    for &i in &idx {
        let (fp, hs, he, sha) = &quads[i];
        let cache_key = (sha.clone(), fp.clone());
        let source = file_cache
            .entry(cache_key)
            .or_insert_with(|| git_show_file(repo_dir, sha, fp));
        let Some(source) = source else { continue };
        let lines: Vec<&str> = source.lines().collect();
        if *he > lines.len() || he <= hs {
            continue;
        }
        let hunk = lines[*hs..*he].join("\n");
        if hunk.trim().is_empty() {
            continue;
        }
        out.push((hunk, repo_dir.join(fp), source.clone()));
    }
    out
}

/// Era-14 phase B: calibration candidates from real diff hunks in
/// `dataset.jsonl`. Lock-step scope with control scoring: excluded paths are
/// dropped, hunks are filtered to the scorer's language, and file content is
/// read at the extraction commit via `git show`. No minimum-size filter — the
/// honesty gain of diff-cal is that tiny hunks calibrate the threshold exactly
/// like the tiny hunks the checker scores.
pub fn collect_diff_candidates(
    dataset_path: &Path,
    repo_dir: &Path,
    language: Language,
) -> Vec<Candidate> {
    #[derive(serde::Deserialize)]
    struct Rec {
        file_path: String,
        hunk_start_line: usize,
        hunk_end_line: usize,
        commit_sha: String,
        language: String,
    }
    let lang_ok = |l: &str| match language {
        Language::Python => l == "python",
        Language::Typescript => l == "typescript",
        Language::Go => l == "go",
        Language::Rust => l == "rust",
        Language::C => l == "c",
        Language::Java => l == "java",
        Language::CSharp => l == "csharp",
        Language::Php => l == "php",
    };
    let raw = match std::fs::read(dataset_path) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut quads: Vec<(String, usize, usize, String)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for line in raw.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let rec: Rec = match serde_json::from_slice(line) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if !lang_ok(&rec.language) {
            continue;
        }
        if is_excluded_path(&repo_dir.join(&rec.file_path), repo_dir) {
            continue;
        }
        let key = format!(
            "{}:{}:{}:{}",
            rec.commit_sha, rec.file_path, rec.hunk_start_line, rec.hunk_end_line
        );
        if seen.insert(key) {
            quads.push((
                rec.file_path,
                rec.hunk_start_line,
                rec.hunk_end_line,
                rec.commit_sha,
            ));
        }
    }
    let mut file_cache: HashMap<(String, String), Option<String>> = HashMap::new();
    let mut out = Vec::new();
    for (fp, hs, he, sha) in &quads {
        let cache_key = (sha.clone(), fp.clone());
        let source = file_cache
            .entry(cache_key)
            .or_insert_with(|| git_show_file(repo_dir, sha, fp));
        let Some(source) = source else { continue };
        let lines: Vec<&str> = source.lines().collect();
        if *he > lines.len() || he <= hs {
            continue;
        }
        let hunk = lines[*hs..*he].join("\n");
        if hunk.trim().is_empty() {
            continue;
        }
        out.push(Candidate {
            hunk,
            file_path: repo_dir.join(fp),
            file_source: source.clone(),
            hunk_start_line: hs + 1,
            hunk_end_line: *he,
        });
    }
    out
}

/// Build a calibrated bench scorer for one (corpus checkout, language).
///
/// `dataset_path` (when present) supplies diff hunks for the auto-detect
/// probe; without it the probe falls back to random source hunks.
pub fn build_scorer(
    repo_dir: &Path,
    language: Language,
    dataset_path: Option<&Path>,
    knobs: &BenchKnobs,
) -> Result<BenchScorer> {
    let adapter = adapter_for(language);
    let files = source_files(repo_dir, language);
    if files.is_empty() {
        bail!(
            "no {:?} source files found in {}",
            language,
            repo_dir.display()
        );
    }
    let repo_files: Vec<(PathBuf, String)> = files
        .iter()
        .filter_map(|p| read_text_lossy(p).ok().map(|s| (p.clone(), s)))
        .collect();

    // Data-dominant filter — mirrors the scorer's own corpus filtering; the
    // probe and calibration call-receivers must see the same corpus the final
    // scorer builds its clusters from.
    let filtered: Vec<(PathBuf, String)> = repo_files
        .iter()
        .filter(|(_, s)| !adapter.is_data_dominant(s))
        .cloned()
        .collect();
    let corpus: &[(PathBuf, String)] = if filtered.is_empty() {
        &repo_files
    } else {
        &filtered
    };

    let typicality = TypicalityModel::new(language);

    // --- Per-corpus auto-detect (era-13.5): probe the cluster-rare fire rate.
    let mut resolved_rare = knobs.cluster_rare_threshold;
    if knobs.auto_select_asym_cal && resolved_rare > 0 && knobs.n_clusters > 1 {
        let probe_hunks = match dataset_path {
            Some(p) if p.exists() => {
                load_diff_hunks_for_probe(p, repo_dir, knobs.asym_probe_n, knobs.seed)
            }
            _ => Vec::new(),
        };
        let probe_hunks = if probe_hunks.is_empty() {
            // Fallback: random source hunks (noisier signal).
            let candidates = collect_candidates(repo_dir, adapter.as_ref());
            let take = knobs.asym_probe_n.min(candidates.len());
            sample_indices(candidates.len(), take, knobs.seed)
                .into_iter()
                .map(|i| {
                    let c = &candidates[i];
                    (c.hunk.clone(), c.file_path.clone(), c.file_source.clone())
                })
                .collect()
        } else {
            probe_hunks
        };

        let mut probe_cr = CallReceiverScorer::new(
            corpus,
            language,
            knobs.alpha,
            knobs.cap,
            adapter.as_ref(),
            knobs.n_clusters,
            knobs.cluster_seed,
            resolved_rare,
            knobs.cluster_size_min,
        )
        .map_err(anyhow::Error::msg)?
        .with_rarity_weighting(knobs.rarity_weighting);

        let mut hunks_scored = 0usize;
        for (hunk, file_path, file_source) in &probe_hunks {
            if typicality.is_atypical(hunk).0 {
                continue;
            }
            probe_cr.weighted_contribution_for_file(
                hunk,
                Some(file_path),
                0.0,
                0.0,
                knobs.cluster_bonus,
                knobs.cap as f64,
                Some(file_source),
                None,
                &Default::default(),
            );
            hunks_scored += 1;
        }
        let fire_rate = probe_cr.rare_branch_hunks_fired as f64 / hunks_scored.max(1) as f64;
        let keep_rule = fire_rate < knobs.asym_fire_rate_threshold;
        eprintln!(
            "[auto-asym] cluster_rare probe: rare_hunks_fired={}/{} fire_rate={:.3} threshold={:.3} → {}",
            probe_cr.rare_branch_hunks_fired,
            hunks_scored,
            fire_rate,
            knobs.asym_fire_rate_threshold,
            if keep_rule {
                "KEEP rule (asym, +catches expected)"
            } else {
                "DISABLE rule (rare=0, baseline)"
            }
        );
        if !keep_rule {
            resolved_rare = 0;
        }
    }

    // --- Multi-seed median threshold (same code path as production calibrate).
    let cal_rare = if knobs.apply_optional_contributions_to_cal {
        resolved_rare
    } else {
        0
    };
    let sources: Vec<String> = corpus.iter().map(|(_, s)| s.clone()).collect();
    let bpe = BpeScorer::new(
        argot_core::bpe::BpeTokenizer::load(),
        argot_core::train::GENERIC_BASELINE_JSON,
        &sources,
    )?;
    let mut cal_cr = CallReceiverScorer::new(
        corpus,
        language,
        knobs.alpha,
        knobs.cap,
        adapter.as_ref(),
        knobs.n_clusters,
        knobs.cluster_seed,
        cal_rare,
        knobs.cluster_size_min,
    )
    .map_err(anyhow::Error::msg)?
    .with_rarity_weighting(knobs.rarity_weighting);
    let candidates = match knobs.calibration_source {
        CalibrationSource::Random => collect_candidates(repo_dir, adapter.as_ref()),
        CalibrationSource::Diff => {
            let ds = dataset_path.context("diff calibration source requires a dataset")?;
            collect_diff_candidates(ds, repo_dir, language)
        }
    };
    if candidates.is_empty() {
        bail!("no calibration candidates in {}", repo_dir.display());
    }
    let effective_n_cal = knobs.n_cal.min(candidates.len());
    let seed_thresholds = multi_seed_thresholds(
        &candidates,
        &bpe,
        &mut cal_cr,
        adapter.as_ref(),
        &typicality,
        &ThresholdRunConfig {
            n_cal: effective_n_cal,
            base_seed: knobs.seed,
            n_seeds: knobs.threshold_n_seeds,
            cluster_bonus: knobs.cluster_bonus,
            cap: knobs.cap as f64,
        },
    );
    let threshold = median(seed_thresholds.clone());

    // --- Era-15 convention-rarity model: corpus frequencies + bars at the
    // max feature value over the multi-seed calibration sample (mirror of
    // run_calibrate; never feeds the threshold per the calibration contract).
    let convention_model = if knobs.enable_conventions {
        // Bars over ALL candidates — deterministic max-gate (see
        // run_calibrate; the two must stay in lock-step).
        let mut model = fit_convention_frequencies(corpus, language);
        let conv = ConventionScorer::new(model.clone(), language);
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
        model.syntax_bar = syntax_bar;
        model.ident_bar = ident_bar;
        Some(model)
    } else {
        None
    };

    // --- Import-module snapshot: corpus imports + repo-owned module names.
    // The retired harness built its scorer with `repo_root=repo_dir`, which
    // folds package/tsconfig aliases into the known-module surface.
    let mut modules: HashSet<String> = HashSet::new();
    for (_, s) in corpus {
        modules.extend(adapter.extract_imports(s));
    }
    let repo_modules = adapter.resolve_repo_modules(repo_dir);
    modules.extend(repo_modules.exact.iter().cloned());
    let mut import_modules: Vec<String> = modules.into_iter().collect();
    import_modules.sort();
    let mut import_module_prefixes: Vec<String> = repo_modules.prefixes.into_iter().collect();
    import_module_prefixes.sort();

    let scorer = SequentialImportBpeScorer::from_config(
        &repo_files,
        argot_core::train::GENERIC_BASELINE_JSON,
        adapter_for(language),
        SequentialConfig {
            bpe_threshold: threshold,
            enable_typicality: knobs.enable_typicality_filter,
            exclude_data_dominant: true,
            call_receiver_alpha: knobs.alpha,
            call_receiver_cap: knobs.cap,
            call_receiver_root_bonus: knobs.root_bonus,
            call_receiver_n_clusters: knobs.n_clusters,
            call_receiver_cluster_seed: knobs.cluster_seed,
            call_receiver_cluster_bonus: knobs.cluster_bonus,
            call_receiver_cluster_rare_threshold: resolved_rare,
            call_receiver_cluster_size_min: knobs.cluster_size_min,
            call_receiver_rarity_weighting: knobs.rarity_weighting,
            call_receiver_shape_primitive_names: knobs.shape_primitive_names.clone(),
            call_receiver_parse_error_host_fallback: knobs.parse_error_host_fallback,
            conventions: convention_model,
            convention_bonus: knobs.convention_bonus,
            import_modules,
            import_module_prefixes,
            evidence_corpus: None,
        },
    )
    .context("building bench scorer")?;

    Ok(BenchScorer {
        scorer,
        seed_thresholds,
        threshold,
        resolved_rare_threshold: resolved_rare,
    })
}
