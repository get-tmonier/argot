//! Repo introspection — `argot inspect`.
//!
//! Answers three questions in one lightweight in-process pass: what would (or
//! did) argot learn from (corpus composition), is the calibration healthy
//! (post-fit, from `.argot/scorer-config.json`), and is this repo suitable at
//! all (verdict). Reuses the calibration inclusion logic (the resolved
//! [`PathSuppressions`] set, [`collect_candidates_with`], the
//! extension→language routing) so what inspect reports is what calibrate
//! would see. Nothing is persisted.

use crate::scoring::adapters::c::CAdapter;
use crate::scoring::adapters::cpp::CppAdapter;
use crate::scoring::adapters::csharp::CSharpAdapter;
use crate::scoring::adapters::go::GoAdapter;
use crate::scoring::adapters::java::JavaAdapter;
use crate::scoring::adapters::php::PhpAdapter;
use crate::scoring::adapters::python::PythonAdapter;
use crate::scoring::adapters::ruby::RubyAdapter;
use crate::scoring::adapters::rust::RustAdapter;
use crate::scoring::adapters::typescript::TypeScriptAdapter;
use crate::scoring::adapters::{Language, LanguageAdapter};
use crate::scoring::calibration::{collect_candidates_with, language_for_filename, language_name};
use crate::suppress::PathSuppressions;
use crate::text::read_text_lossy;
use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

// --- suitability thresholds ---
//
// Derived from the bench-config history (the pinned corpora in
// `benchmarks/targets.yaml`, whose calibrations sample n_cal = 100–500 hunks
// per seed) and the production calibration defaults
// (`CalibrateOptions::default()`: n_cal = 500, 7 seeds). The calibrated
// threshold is a max over sampled candidate scores, so with only a few dozen
// candidates it is dominated by a handful of hunks and swings with the seed.

/// Candidate hunks below which the calibrated threshold is seed-sensitive
/// (yellow signal).
pub const RECOMMENDED_CANDIDATE_HUNKS: usize = 200;
/// Candidate hunks below which calibration is unstable (red signal).
pub const MIN_CANDIDATE_HUNKS: usize = 50;
/// A minority supported language above this share of supported files makes
/// the repo "meaningfully mixed" (yellow signal: multi-language calibration
/// is less mature than single-language calibration).
pub const POLYGLOT_MINORITY_SHARE: f64 = 0.20;

/// Suitability verdict for the inspected repo.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Ready,
    Marginal,
    NotRecommended,
}

/// Severity of a suitability reason.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReasonLevel {
    Yellow,
    Red,
}

/// One suitability finding: the signal that fired and a human message citing
/// the numbers behind it.
#[derive(Serialize, Debug, Clone)]
pub struct Reason {
    pub level: ReasonLevel,
    pub signal: String,
    pub message: String,
}

/// Corpus composition for one supported language.
#[derive(Serialize, Debug, Clone, Default)]
pub struct LanguageCorpus {
    /// Files routed to this language by extension.
    pub files: usize,
    /// Files calibration would draw candidates from.
    pub included: usize,
    /// Excluded by the resolved path-suppression set (recommended built-ins
    /// plus `.argotignore`).
    pub excluded_path: usize,
    /// Excluded because the file header carries an auto-generation marker.
    pub auto_generated: usize,
    /// Excluded because the file is dominated by static data literals.
    pub data_dominant: usize,
    /// Share of all supported files routed to this language (0.0–1.0).
    pub share_of_supported: f64,
    /// Calibration candidate hunks ([`collect_candidates_with`] — the exact
    /// population calibrate samples from).
    pub candidate_hunks: usize,
}

/// What argot would learn from: per-language composition plus polyglotism.
#[derive(Serialize, Debug, Clone)]
pub struct CorpusReport {
    /// All files walked (excluding `.git` internals).
    pub total_files: usize,
    /// Files with a supported extension.
    pub supported_files: usize,
    /// Files with an unsupported extension (excluded from the corpus).
    pub unsupported_files: usize,
    /// Per-language breakdown, keyed by config-key name.
    pub languages: BTreeMap<String, LanguageCorpus>,
    /// True when at least two supported languages each exceed
    /// [`POLYGLOT_MINORITY_SHARE`] of supported files.
    pub meaningfully_mixed: bool,
}

/// Calibration health for one language, from `scorer-config.json` (v3) plus
/// the live candidate pass.
#[derive(Serialize, Debug, Clone)]
pub struct LanguageCalibration {
    pub threshold: f64,
    pub n_cal: usize,
    pub seed: u64,
    pub n_seeds: usize,
    pub repo_sha: String,
    pub timestamp_utc: String,
    /// Candidate hunks available in the working tree right now — compare with
    /// `n_cal` to see how much the sampleable population has drifted.
    pub candidate_hunks_now: usize,
    /// The BPE stage's reachable ceiling under this model: the maximum
    /// surprise any single token can score (a hunk's BPE score is a max over
    /// its tokens). Computed from the fit-time model snapshot.
    pub bpe_ceiling: f64,
    /// The call-receiver contribution cap in score points.
    pub contribution_cap: f64,
    /// `bpe_ceiling + contribution_cap − threshold`: how far the strongest
    /// possible import-free hunk clears the threshold. ≤ 0 means phrasing
    /// detection cannot fire — the fit is an import tripwire only.
    pub phrasing_headroom: f64,
}

/// Post-fit calibration health (absent when `.argot/scorer-config.json` does
/// not exist).
#[derive(Serialize, Debug, Clone)]
pub struct CalibrationReport {
    pub config_path: String,
    pub languages: BTreeMap<String, LanguageCalibration>,
}

/// The full `argot inspect` report (serialized as-is by `--json`).
#[derive(Serialize, Debug, Clone)]
pub struct InspectReport {
    pub path: String,
    pub corpus: CorpusReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration: Option<CalibrationReport>,
    pub verdict: Verdict,
    pub reasons: Vec<Reason>,
}

/// Artifact-level fields from `.argot/manifest.json` (`argot inspect --model`).
#[derive(Serialize, Debug, Clone)]
pub struct ManifestView {
    pub manifest_version: u64,
    pub config_version: u64,
    pub model_hash: String,
    pub scorer_config_hash: String,
    pub fit_commit_sha: String,
    pub fit_timestamp: String,
    pub corpus_files: usize,
    pub corpus_lines: usize,
}

/// One callee-bag cluster in the fitted model, with its most-typical callees.
#[derive(Serialize, Debug, Clone)]
pub struct ClusterView {
    pub id: String,
    pub files: usize,
    /// Top callees by how many cluster files use them (name, file count).
    pub top_callees: Vec<(String, usize)>,
}

/// Per-language view of the fit-time model snapshot.
#[derive(Serialize, Debug, Clone)]
pub struct LanguageModelView {
    pub threshold: f64,
    pub model_hash: String,
    pub n_cal: usize,
    pub distinct_tokens: usize,
    pub total_tokens: u64,
    pub attested_callees: usize,
    pub corpus_files: usize,
    /// The repo's familiar import surface (first-party + attested third-party).
    pub familiar_imports: Vec<String>,
    pub clusters: Vec<ClusterView>,
}

/// `argot inspect --model` report: what the model learned, plus the artifact
/// provenance that lets two fits be compared.
#[derive(Serialize, Debug, Clone)]
pub struct ModelReport {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<ManifestView>,
    pub languages: BTreeMap<String, LanguageModelView>,
}

fn adapter_for(language: Language) -> Box<dyn LanguageAdapter> {
    match language {
        Language::Python => Box::new(PythonAdapter::new()),
        Language::Typescript => Box::new(TypeScriptAdapter::new()),
        Language::Go => Box::new(GoAdapter::new()),
        Language::Rust => Box::new(RustAdapter::new()),
        Language::C => Box::new(CAdapter::new()),
        Language::Java => Box::new(JavaAdapter::new()),
        Language::CSharp => Box::new(CSharpAdapter::new()),
        Language::Php => Box::new(PhpAdapter::new()),
        Language::Cpp => Box::new(CppAdapter::new()),
        Language::Ruby => Box::new(RubyAdapter::new()),
    }
}

/// Walk the repo and classify every file the way calibration would: extension
/// routing, then the resolved path-suppression set (recommended built-ins +
/// `.argotignore` — the same set calibrate and check consult), then the
/// structural filters.
fn scan_corpus(repo_dir: &Path) -> CorpusReport {
    let path_suppressions = PathSuppressions::load(repo_dir);
    let mut total_files = 0usize;
    let mut unsupported_files = 0usize;
    let mut languages: BTreeMap<String, LanguageCorpus> = BTreeMap::new();
    let mut adapters: BTreeMap<&'static str, Box<dyn LanguageAdapter>> = BTreeMap::new();

    let mut stack = vec![repo_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            match entry.file_type() {
                Ok(t) if t.is_dir() => {
                    // `.git` internals are repository plumbing, not corpus;
                    // everything else is walked so the numbers match what
                    // `collect_candidates` sees.
                    if name != ".git" {
                        stack.push(path);
                    }
                }
                Ok(t) if t.is_file() => {
                    total_files += 1;
                    let language = match language_for_filename(&name) {
                        Some(l) => l,
                        None => {
                            unsupported_files += 1;
                            continue;
                        }
                    };
                    let key = language_name(language);
                    let stats = languages.entry(key.to_string()).or_default();
                    stats.files += 1;
                    if path_suppressions.is_suppressed_abs(&path, repo_dir) {
                        stats.excluded_path += 1;
                        continue;
                    }
                    let source = match read_text_lossy(&path) {
                        Ok(s) => s,
                        Err(_) => {
                            stats.excluded_path += 1;
                            continue;
                        }
                    };
                    let adapter = adapters.entry(key).or_insert_with(|| adapter_for(language));
                    if adapter.is_auto_generated(&source) {
                        stats.auto_generated += 1;
                    } else if adapter.is_data_dominant(&source) {
                        stats.data_dominant += 1;
                    } else {
                        stats.included += 1;
                    }
                }
                _ => {}
            }
        }
    }

    let supported_files: usize = languages.values().map(|l| l.files).sum();

    // Candidate hunks per language present in the corpus — the exact
    // population calibrate samples thresholds from.
    for (key, stats) in &mut languages {
        let language = match key.as_str() {
            "python" => Language::Python,
            "go" => Language::Go,
            "rust" => Language::Rust,
            "c" => Language::C,
            "java" => Language::Java,
            "csharp" => Language::CSharp,
            "php" => Language::Php,
            "cpp" => Language::Cpp,
            "ruby" => Language::Ruby,
            _ => Language::Typescript,
        };
        let adapter = adapters
            .entry(language_name(language))
            .or_insert_with(|| adapter_for(language));
        stats.candidate_hunks =
            collect_candidates_with(repo_dir, adapter.as_ref(), &path_suppressions).len();
        stats.share_of_supported = if supported_files == 0 {
            0.0
        } else {
            stats.files as f64 / supported_files as f64
        };
    }

    let meaningfully_mixed = languages
        .values()
        .filter(|l| l.share_of_supported > POLYGLOT_MINORITY_SHARE)
        .count()
        >= 2;

    CorpusReport {
        total_files,
        supported_files,
        unsupported_files,
        languages,
        meaningfully_mixed,
    }
}

/// Read calibration health from a v3 `scorer-config.json`.
///
/// `Ok(None)` = no config (pre-fit). `Err(msg)` = config present but not a
/// readable v3 document (surfaced as a yellow reason, never a hard failure —
/// inspect must not die on the thing it diagnoses).
fn load_calibration(
    config_path: &Path,
    corpus: &CorpusReport,
) -> std::result::Result<Option<CalibrationReport>, String> {
    let bytes = match std::fs::read(config_path) {
        Ok(b) => b,
        Err(_) => return Ok(None),
    };
    let config: Value = serde_json::from_slice(&bytes)
        .map_err(|e| format!("{} is not valid JSON: {e}", config_path.display()))?;
    if config.get("version").and_then(Value::as_i64) != Some(3) {
        return Err(format!(
            "{} is not a v3 scorer config — regenerate via `argot fit`",
            config_path.display()
        ));
    }
    let langs = config
        .get("languages")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{} is missing the 'languages' block", config_path.display()))?;

    let mut out: BTreeMap<String, LanguageCalibration> = BTreeMap::new();
    for (lang, cfg) in langs {
        let threshold = cfg
            .get("threshold")
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("language '{lang}' has no numeric 'threshold'"))?;
        let cal = cfg.get("calibration").cloned().unwrap_or(Value::Null);
        let get_usize = |k: &str| cal.get(k).and_then(Value::as_u64).unwrap_or(0) as usize;
        let get_str = |k: &str| {
            cal.get(k)
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string()
        };

        // Phrasing-detection headroom from the fit-time model snapshot: what
        // can the strongest import-free hunk actually score against this
        // threshold?
        let bpe_stats: crate::scoring::model::BpeStats = cfg
            .get("model")
            .and_then(|m| m.get("bpe"))
            .and_then(|b| serde_json::from_value(b.clone()).ok())
            .ok_or_else(|| format!("language '{lang}' has no readable model.bpe block"))?;
        let bpe = crate::scoring::bpe_scorer::BpeScorer::from_stats(
            crate::bpe::BpeTokenizer::load(),
            crate::train::GENERIC_BASELINE_JSON,
            &bpe_stats,
        )
        .map_err(|e| format!("language '{lang}': cannot rebuild BPE stats: {e}"))?;
        let bpe_ceiling = bpe.max_token_surprise_ceiling();
        let contribution_cap = cfg
            .get("call_receiver_cap")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        out.insert(
            lang.clone(),
            LanguageCalibration {
                threshold,
                n_cal: get_usize("n_cal"),
                seed: cal.get("seed").and_then(Value::as_u64).unwrap_or(0),
                n_seeds: get_usize("n_seeds"),
                repo_sha: get_str("repo_sha"),
                timestamp_utc: get_str("timestamp_utc"),
                candidate_hunks_now: corpus
                    .languages
                    .get(lang)
                    .map(|l| l.candidate_hunks)
                    .unwrap_or(0),
                bpe_ceiling,
                contribution_cap,
                phrasing_headroom: bpe_ceiling + contribution_cap - threshold,
            },
        );
    }
    Ok(Some(CalibrationReport {
        config_path: config_path.display().to_string(),
        languages: out,
    }))
}

/// Suitability reasons derived from the calibration's phrasing headroom.
/// Red when phrasing detection cannot fire at all (import tripwire only);
/// yellow when alien phrasing alone cannot cross and detection depends on
/// unattested-callee contributions.
pub fn phrasing_reasons(cal: &CalibrationReport) -> Vec<Reason> {
    let mut reasons = Vec::new();
    for (lang, lc) in &cal.languages {
        if lc.phrasing_headroom <= 0.0 {
            reasons.push(Reason {
                level: ReasonLevel::Red,
                signal: "phrasing_detection_dead".to_string(),
                message: format!(
                    "{lang}: threshold {:.2} exceeds the strongest import-free hunk \
                     (BPE ceiling {:.2} + callee cap {:.0} = {:.2}) — phrasing \
                     detection cannot fire; argot is an import tripwire on this fit",
                    lc.threshold,
                    lc.bpe_ceiling,
                    lc.contribution_cap,
                    lc.bpe_ceiling + lc.contribution_cap
                ),
            });
        } else if lc.threshold > lc.bpe_ceiling {
            reasons.push(Reason {
                level: ReasonLevel::Yellow,
                signal: "phrasing_needs_callees".to_string(),
                message: format!(
                    "{lang}: threshold {:.2} exceeds the BPE ceiling {:.2} — alien \
                     phrasing alone cannot cross; detection relies on unattested-callee \
                     contributions (headroom {:.2})",
                    lc.threshold, lc.bpe_ceiling, lc.phrasing_headroom
                ),
            });
        }
    }
    reasons
}

/// Compute the suitability verdict from corpus composition. Every yellow/red
/// reason cites the signal that fired and the numbers behind it.
pub fn compute_verdict(corpus: &CorpusReport) -> (Verdict, Vec<Reason>) {
    let mut reasons: Vec<Reason> = Vec::new();

    if corpus.supported_files == 0 {
        reasons.push(Reason {
            level: ReasonLevel::Red,
            signal: "no_supported_files".to_string(),
            message: format!(
                "no supported-language files found ({} files scanned) — \
                 argot has nothing to learn a voice from",
                corpus.total_files
            ),
        });
    }

    for (lang, stats) in &corpus.languages {
        // A trace language (a handful of stray files in an otherwise
        // single-language repo) doesn't get calibrated and shouldn't tank
        // the verdict; the candidate-hunk signals apply only to languages
        // that meaningfully compose the repo.
        let share = if corpus.supported_files > 0 {
            stats.files as f64 / corpus.supported_files as f64
        } else {
            0.0
        };
        if share < POLYGLOT_MINORITY_SHARE {
            continue;
        }
        if stats.candidate_hunks < MIN_CANDIDATE_HUNKS {
            reasons.push(Reason {
                level: ReasonLevel::Red,
                signal: "low_candidate_hunks".to_string(),
                message: format!(
                    "{lang}: {} calibration candidate hunks — too few hunks to \
                     calibrate stably; recommended ≥ {RECOMMENDED_CANDIDATE_HUNKS}",
                    stats.candidate_hunks
                ),
            });
        } else if stats.candidate_hunks < RECOMMENDED_CANDIDATE_HUNKS {
            reasons.push(Reason {
                level: ReasonLevel::Yellow,
                signal: "low_candidate_hunks".to_string(),
                message: format!(
                    "{lang}: {} calibration candidate hunks — below the \
                     ≥ {RECOMMENDED_CANDIDATE_HUNKS} recommended for a \
                     seed-stable threshold",
                    stats.candidate_hunks
                ),
            });
        }
    }

    if corpus.meaningfully_mixed {
        let shares = format_shares(corpus);
        reasons.push(Reason {
            level: ReasonLevel::Yellow,
            signal: "polyglot_mix".to_string(),
            message: format!(
                "meaningfully mixed corpus ({shares}) — each language \
                 calibrates on its own smaller corpus, and multi-language \
                 calibration is less mature than single-language"
            ),
        });
    }

    (verdict_from_reasons(&reasons), reasons)
}

/// Any red → not recommended; any reason at all → marginal; else ready.
pub fn verdict_from_reasons(reasons: &[Reason]) -> Verdict {
    if reasons.iter().any(|r| r.level == ReasonLevel::Red) {
        Verdict::NotRecommended
    } else if reasons.is_empty() {
        Verdict::Ready
    } else {
        Verdict::Marginal
    }
}

/// Polyglotism ratio, largest share first: "73% typescript · 27% python".
pub fn format_shares(corpus: &CorpusReport) -> String {
    let mut shares: Vec<(&String, f64)> = corpus
        .languages
        .iter()
        .map(|(k, l)| (k, l.share_of_supported))
        .collect();
    shares.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    shares
        .iter()
        .map(|(k, s)| format!("{:.0}% {}", s * 100.0, k))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Inspect `repo_dir`: corpus composition, calibration health when
/// `.argot/scorer-config.json` exists, and the suitability verdict.
pub fn inspect_repo(repo_dir: &Path) -> Result<InspectReport> {
    if !repo_dir.is_dir() {
        anyhow::bail!("not a directory: {}", repo_dir.display());
    }
    let corpus = scan_corpus(repo_dir);
    let (_, mut reasons) = compute_verdict(&corpus);

    let config_path = repo_dir.join(".argot").join("scorer-config.json");
    let calibration = match load_calibration(&config_path, &corpus) {
        Ok(c) => {
            if let Some(cal) = &c {
                reasons.extend(phrasing_reasons(cal));
            }
            c
        }
        Err(msg) => {
            reasons.push(Reason {
                level: ReasonLevel::Yellow,
                signal: "scorer_config_invalid".to_string(),
                message: msg,
            });
            None
        }
    };

    Ok(InspectReport {
        path: repo_dir.display().to_string(),
        verdict: verdict_from_reasons(&reasons),
        corpus,
        calibration,
        reasons,
    })
}

/// Read the fitted model artifact for `argot inspect --model`: the manifest
/// provenance plus, per language, the BPE/callee summary and the most-typical
/// callees in each cluster (`top_n` per cluster). Errors only when there's no
/// scorer-config to read.
pub fn inspect_model(repo_dir: &Path, top_n: usize) -> Result<ModelReport> {
    use crate::scoring::model::LanguageModel;

    let argot_dir = repo_dir.join(".argot");
    let config_path = argot_dir.join("scorer-config.json");
    let bytes = std::fs::read(&config_path).map_err(|_| {
        anyhow::anyhow!(
            "no fitted model at {} — run `argot fit` first",
            config_path.display()
        )
    })?;
    let config: Value = serde_json::from_slice(&bytes)
        .map_err(|e| anyhow::anyhow!("{} is not valid JSON: {e}", config_path.display()))?;

    // Manifest is optional — a fit that predates it still has a readable model.
    let str_field = |v: &Value, k: &str, default: &str| {
        v.get(k)
            .and_then(Value::as_str)
            .unwrap_or(default)
            .to_string()
    };
    let manifest = std::fs::read(argot_dir.join("manifest.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .map(|m| ManifestView {
            manifest_version: m
                .get("manifest_version")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            config_version: m.get("config_version").and_then(Value::as_u64).unwrap_or(0),
            model_hash: str_field(&m, "model_hash", ""),
            scorer_config_hash: str_field(&m, "scorer_config_hash", ""),
            fit_commit_sha: str_field(&m, "fit_commit_sha", "unknown"),
            fit_timestamp: str_field(&m, "fit_timestamp", "unknown"),
            corpus_files: m
                .get("corpus")
                .and_then(|c| c.get("files"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            corpus_lines: m
                .get("corpus")
                .and_then(|c| c.get("lines"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
        });

    let mut languages: BTreeMap<String, LanguageModelView> = BTreeMap::new();
    if let Some(langs) = config.get("languages").and_then(Value::as_object) {
        for (lang, cfg) in langs {
            let Some(model) = cfg
                .get("model")
                .and_then(|m| serde_json::from_value::<LanguageModel>(m.clone()).ok())
            else {
                continue;
            };
            let mut clusters: Vec<ClusterView> = model
                .call_receiver
                .clusters
                .iter()
                .map(|(id, cm)| {
                    let mut callees: Vec<(String, usize)> = cm
                        .callee_counts
                        .iter()
                        .map(|(c, n)| (c.clone(), *n))
                        .collect();
                    // Most-typical first; ties broken by name for determinism.
                    callees.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                    callees.truncate(top_n);
                    ClusterView {
                        id: id.clone(),
                        files: cm.files.len(),
                        top_callees: callees,
                    }
                })
                .collect();
            clusters.sort_by(|a, b| a.id.cmp(&b.id));
            languages.insert(
                lang.clone(),
                LanguageModelView {
                    threshold: cfg.get("threshold").and_then(Value::as_f64).unwrap_or(0.0),
                    model_hash: str_field(cfg, "model_hash", ""),
                    n_cal: cfg
                        .get("calibration")
                        .and_then(|c| c.get("n_cal"))
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize,
                    distinct_tokens: model.bpe.token_counts.len(),
                    total_tokens: model.bpe.total_tokens,
                    attested_callees: model.call_receiver.attested.len(),
                    corpus_files: model.call_receiver.n_corpus_files,
                    familiar_imports: cfg
                        .get("import_modules")
                        .and_then(Value::as_array)
                        .map(|a| {
                            a.iter()
                                .filter_map(Value::as_str)
                                .map(String::from)
                                .collect()
                        })
                        .unwrap_or_default(),
                    clusters,
                },
            );
        }
    }

    Ok(ModelReport {
        path: repo_dir.display().to_string(),
        manifest,
        languages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_repo(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("argot_inspect_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn py_functions(n: usize) -> String {
        (0..n)
            .map(|i| {
                format!(
                    "def compute_{i}(a, b):\n    x = a + b\n    y = x * 2\n    \
                     z = y - a\n    w = z + 1\n    v = w * 3\n    u = v - b\n    return u\n"
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn ts_functions(n: usize) -> String {
        (0..n)
            .map(|i| {
                format!(
                    "export function compute{i}(a: number, b: number): number {{\n  \
                     const x = a + b;\n  const y = x * 2;\n  const z = y - a;\n  \
                     const w = z + 1;\n  const v = w * 3;\n  return v - b;\n}}\n"
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn empty_repo_is_not_recommended() {
        let dir = temp_repo("empty");
        fs::write(dir.join("README.md"), "hello").unwrap();
        let report = inspect_repo(&dir).unwrap();
        assert_eq!(report.verdict, Verdict::NotRecommended);
        assert!(report
            .reasons
            .iter()
            .any(|r| r.signal == "no_supported_files" && r.level == ReasonLevel::Red));
        assert_eq!(report.corpus.supported_files, 0);
        assert_eq!(report.corpus.unsupported_files, 1);
        assert!(report.calibration.is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn tiny_repo_is_not_recommended_for_low_candidates() {
        let dir = temp_repo("tiny");
        fs::write(dir.join("app.py"), py_functions(3)).unwrap();
        let report = inspect_repo(&dir).unwrap();
        let py = &report.corpus.languages["python"];
        assert_eq!(py.candidate_hunks, 3);
        assert_eq!(report.verdict, Verdict::NotRecommended);
        let reason = report
            .reasons
            .iter()
            .find(|r| r.signal == "low_candidate_hunks")
            .expect("low_candidate_hunks reason");
        assert_eq!(reason.level, ReasonLevel::Red);
        assert!(reason.message.contains("3 calibration candidate hunks"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mid_size_repo_is_marginal() {
        let dir = temp_repo("mid");
        fs::write(dir.join("app.py"), py_functions(80)).unwrap();
        let report = inspect_repo(&dir).unwrap();
        assert_eq!(report.corpus.languages["python"].candidate_hunks, 80);
        assert_eq!(report.verdict, Verdict::Marginal);
        let reason = report
            .reasons
            .iter()
            .find(|r| r.signal == "low_candidate_hunks")
            .expect("low_candidate_hunks reason");
        assert_eq!(reason.level, ReasonLevel::Yellow);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn healthy_single_language_repo_is_ready() {
        let dir = temp_repo("healthy");
        for f in 0..5 {
            fs::write(dir.join(format!("mod_{f}.py")), py_functions(50)).unwrap();
        }
        let report = inspect_repo(&dir).unwrap();
        let py = &report.corpus.languages["python"];
        assert_eq!(py.files, 5);
        assert_eq!(py.included, 5);
        assert_eq!(py.candidate_hunks, 250);
        assert!((py.share_of_supported - 1.0).abs() < 1e-9);
        assert_eq!(report.verdict, Verdict::Ready);
        assert!(report.reasons.is_empty());
        assert!(!report.corpus.meaningfully_mixed);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn polyglot_mix_is_marginal_with_polyglot_reason() {
        let dir = temp_repo("polyglot");
        for f in 0..5 {
            fs::write(dir.join(format!("mod_{f}.py")), py_functions(50)).unwrap();
            fs::write(dir.join(format!("mod_{f}.ts")), ts_functions(50)).unwrap();
        }
        let report = inspect_repo(&dir).unwrap();
        assert!(report.corpus.meaningfully_mixed);
        assert_eq!(report.verdict, Verdict::Marginal);
        let reason = report
            .reasons
            .iter()
            .find(|r| r.signal == "polyglot_mix")
            .expect("polyglot_mix reason");
        assert_eq!(reason.level, ReasonLevel::Yellow);
        assert!(reason.message.contains("50% python"));
        assert!(reason.message.contains("50% typescript"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn exclusion_reasons_are_counted() {
        let dir = temp_repo("exclusions");
        fs::create_dir_all(dir.join("tests")).unwrap();
        fs::write(dir.join("app.py"), py_functions(2)).unwrap();
        fs::write(dir.join("tests/test_app.py"), py_functions(1)).unwrap();
        fs::write(
            dir.join("generated.py"),
            format!(
                "# This file is auto-generated. Do not edit.\n{}",
                py_functions(1)
            ),
        )
        .unwrap();
        fs::write(
            dir.join("data.py"),
            "TABLE = {\n    \"a\": 1,\n    \"b\": 2,\n    \"c\": 3,\n    \"d\": 4,\n    \"e\": 5,\n}\n",
        )
        .unwrap();
        let report = inspect_repo(&dir).unwrap();
        let py = &report.corpus.languages["python"];
        assert_eq!(py.files, 4);
        assert_eq!(py.included, 1);
        assert_eq!(py.excluded_path, 1, "tests/ file excluded by path");
        assert_eq!(py.auto_generated, 1);
        assert_eq!(py.data_dominant, 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn calibration_health_is_read_from_v3_config() {
        let dir = temp_repo("calibrated");
        fs::write(dir.join("app.py"), py_functions(10)).unwrap();
        fs::create_dir_all(dir.join(".argot")).unwrap();
        fs::write(
            dir.join(".argot/scorer-config.json"),
            r#"{
              "version": 3,
              "languages": {
                "python": {
                  "threshold": 42.5,
                  "call_receiver_cap": 5,
                  "calibration": {
                    "n_cal": 9,
                    "seed": 0,
                    "n_seeds": 7,
                    "repo_sha": "deadbeef",
                    "timestamp_utc": "1970-01-01T00:00:00+00:00"
                  },
                  "model": {
                    "bpe": { "token_counts": {}, "total_tokens": 0 },
                    "call_receiver": { "attested": [], "n_corpus_files": 0, "clusters": {} }
                  }
                }
              }
            }"#,
        )
        .unwrap();
        let report = inspect_repo(&dir).unwrap();
        let cal = report.calibration.expect("calibration report");
        let py = &cal.languages["python"];
        assert_eq!(py.threshold, 42.5);
        assert_eq!(py.n_cal, 9);
        assert_eq!(py.n_seeds, 7);
        assert_eq!(py.seed, 0);
        assert_eq!(py.repo_sha, "deadbeef");
        assert_eq!(py.timestamp_utc, "1970-01-01T00:00:00+00:00");
        assert_eq!(py.candidate_hunks_now, 10, "live candidate pass");
        // Threshold 42.5 is far above any reachable token surprise → the
        // phrasing-dead red reason fires and the verdict is honest about it.
        assert!(py.bpe_ceiling > 0.0);
        assert!(py.phrasing_headroom < 0.0);
        assert!(report
            .reasons
            .iter()
            .any(|r| r.signal == "phrasing_detection_dead" && r.level == ReasonLevel::Red));
        assert_eq!(report.verdict, Verdict::NotRecommended);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn healthy_threshold_has_phrasing_headroom() {
        let dir = temp_repo("headroom");
        fs::write(dir.join("app.py"), py_functions(10)).unwrap();
        fs::create_dir_all(dir.join(".argot")).unwrap();
        fs::write(
            dir.join(".argot/scorer-config.json"),
            r#"{
              "version": 3,
              "languages": {
                "python": {
                  "threshold": 4.5,
                  "call_receiver_cap": 5,
                  "calibration": {},
                  "model": {
                    "bpe": { "token_counts": {}, "total_tokens": 0 },
                    "call_receiver": { "attested": [], "n_corpus_files": 0, "clusters": {} }
                  }
                }
              }
            }"#,
        )
        .unwrap();
        let report = inspect_repo(&dir).unwrap();
        let cal = report.calibration.expect("calibration report");
        let py = &cal.languages["python"];
        assert!(
            py.phrasing_headroom > 0.0,
            "headroom {}",
            py.phrasing_headroom
        );
        assert!(!report
            .reasons
            .iter()
            .any(|r| r.signal.starts_with("phrasing")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_config_yields_yellow_reason_not_error() {
        let dir = temp_repo("badconfig");
        fs::write(dir.join("app.py"), py_functions(1)).unwrap();
        fs::create_dir_all(dir.join(".argot")).unwrap();
        fs::write(dir.join(".argot/scorer-config.json"), "{ not json").unwrap();
        let report = inspect_repo(&dir).unwrap();
        assert!(report.calibration.is_none());
        assert!(report
            .reasons
            .iter()
            .any(|r| r.signal == "scorer_config_invalid" && r.level == ReasonLevel::Yellow));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_document_shape_is_stable() {
        let dir = temp_repo("jsonshape");
        fs::write(dir.join("app.py"), py_functions(1)).unwrap();
        let report = inspect_repo(&dir).unwrap();
        let json: Value = serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();
        assert_eq!(json["verdict"], "not_recommended");
        assert!(json["reasons"].as_array().unwrap().iter().all(|r| {
            r.get("level").is_some() && r.get("signal").is_some() && r.get("message").is_some()
        }));
        let py = &json["corpus"]["languages"]["python"];
        for key in [
            "files",
            "included",
            "excluded_path",
            "auto_generated",
            "data_dominant",
            "share_of_supported",
            "candidate_hunks",
        ] {
            assert!(py.get(key).is_some(), "missing corpus key {key}");
        }
        assert!(json.get("calibration").is_none(), "pre-fit: no calibration");
        let _ = fs::remove_dir_all(&dir);
    }
}
