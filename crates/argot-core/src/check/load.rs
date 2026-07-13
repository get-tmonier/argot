//! Per-language scorer loading from `.argot/scorer-config.json`.

use super::{ext_to_lang, extension, PatchBatch, EXT_TO_LANG};
use crate::config::DetectConfig;
use crate::scoring::adapters::c::CAdapter;
use crate::scoring::adapters::cpp::CppAdapter;
use crate::scoring::adapters::csharp::CSharpAdapter;
use crate::scoring::adapters::go::GoAdapter;
use crate::scoring::adapters::java::JavaAdapter;
use crate::scoring::adapters::javascript::JavaScriptAdapter;
use crate::scoring::adapters::php::PhpAdapter;
use crate::scoring::adapters::python::PythonAdapter;
use crate::scoring::adapters::ruby::RubyAdapter;
use crate::scoring::adapters::rust::RustAdapter;
use crate::scoring::adapters::typescript::TypeScriptAdapter;
use crate::scoring::adapters::LanguageAdapter;
use crate::scoring::evidence::types::EvidenceCorpus;
use crate::scoring::model::LanguageModel;
use crate::scoring::sequential::{ScoredHunk, SequentialConfig, SequentialImportBpeScorer};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// One calibrated slice for check-time dispatch: its threshold applies to hunks
/// whose repo-relative path matches any of `paths`.
pub(super) struct SliceEntry {
    pub(super) paths: Vec<String>,
    pub(super) threshold: f64,
}
/// Loaded per-language scorers plus the filtering machinery.
pub(super) struct Loaded {
    pub(super) scorers: HashMap<String, SequentialImportBpeScorer>,
    pub(super) filter_adapters: HashMap<String, Box<dyn LanguageAdapter>>,
    pub(super) language_extensions: HashSet<String>,
    /// Per-language slice thresholds (per-subdirectory / per-author voice).
    /// Empty for an unsliced fit.
    pub(super) slices: HashMap<String, Vec<SliceEntry>>,
    /// Per-language new-file thresholds. A hunk whose file was absent from the
    /// fit corpus is judged against this (higher) bar instead of `threshold`
    /// (issue #92 new-file flooding). Absent for configs predating the field —
    /// then new files keep the single-threshold behaviour.
    pub(super) new_file_thresholds: HashMap<String, f64>,
    /// Authoritative fit-corpus file set (repo-relative), including data-dominant
    /// files. A path absent here is a new file. Empty for configs predating the
    /// field — then new-file detection falls back to cluster membership, which
    /// misclassifies data-dominant known files (issue #92).
    pub(super) fit_corpus_files: HashSet<String>,
    /// Repo SHA the model was fitted at (calibration meta), for the
    /// freshness warning. `None` when the config predates the field.
    pub(super) fit_sha: Option<String>,
    /// Combined fingerprint of the fit-time model — the same `model_hash` the
    /// manifest records. Lets `check` name which model judged the diff.
    pub(super) model_hash: String,
}
fn adapter_for_language(lang: &str) -> Option<Box<dyn LanguageAdapter>> {
    match lang {
        "python" => Some(Box::new(PythonAdapter::new())),
        "typescript" => Some(Box::new(TypeScriptAdapter::new())),
        "javascript" => Some(Box::new(JavaScriptAdapter::new())),
        "go" => Some(Box::new(GoAdapter::new())),
        "rust" => Some(Box::new(RustAdapter::new())),
        "c" => Some(Box::new(CAdapter::new())),
        "java" => Some(Box::new(JavaAdapter::new())),
        "csharp" => Some(Box::new(CSharpAdapter::new())),
        "php" => Some(Box::new(PhpAdapter::new())),
        "cpp" => Some(Box::new(CppAdapter::new())),
        "ruby" => Some(Box::new(RubyAdapter::new())),
        _ => None,
    }
}
/// Languages present in the change that argot supports but the current fit
/// has no model for (fitted before the language appeared in the repo).
pub(super) fn patches_langs_without_model(
    patches: &[PatchBatch],
    scorers: &HashMap<String, SequentialImportBpeScorer>,
) -> Vec<&'static str> {
    patches
        .iter()
        .filter_map(|b| ext_to_lang(&extension(&b.file_path)))
        .filter(|lang| !scorers.contains_key(*lang))
        .collect()
}
/// Best-effort Python `repr` of the `version` value for the mismatch message.
fn py_repr(v: &Value) -> String {
    match v {
        Value::Null => "None".to_string(),
        Value::String(s) => format!("'{s}'"),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => {
            if *b {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        other => other.to_string(),
    }
}
/// Load v3 per-language scorers from `.argot/` — entirely from the fit-time
/// model snapshot in `scorer-config.json`. The live tree is never consulted:
/// scoring must judge new code against the voice as it was learned, not as
/// the new code just rewrote it (issue #79).
/// On failure returns the exact stderr message and exit code.
pub(super) fn load_scorers(
    argot_dir: &Path,
    detect: &DetectConfig,
) -> Result<Loaded, (String, i32)> {
    let generic_baseline_json = argot_dir.join("generic-baseline.json");
    let config_json = argot_dir.join("scorer-config.json");

    for (p, msg) in [
        (&generic_baseline_json, "run `argot fit` first"),
        (&config_json, "run `argot calibrate` first"),
    ] {
        if !p.exists() {
            return Err((format!("error: {} not found — {}\n", p.display(), msg), 2));
        }
    }

    let config_bytes = fs::read(&config_json).map_err(|e| (format!("error: {e}\n"), 2))?;
    let config: Value =
        serde_json::from_slice(&config_bytes).map_err(|e| (format!("error: {e}\n"), 2))?;

    if config.get("version").and_then(Value::as_i64) != Some(3) {
        let vrepr = config
            .get("version")
            .map(py_repr)
            .unwrap_or_else(|| "None".to_string());
        return Err((
            format!(
                "error: {} uses config version {} — regenerate via `argot fit`.\n",
                config_json.display(),
                vrepr
            ),
            2,
        ));
    }

    let languages = match config.get("languages").and_then(Value::as_object) {
        Some(m) => m,
        None => {
            return Err((
                format!(
                    "error: {} is missing the 'languages' block\n",
                    config_json.display()
                ),
                2,
            ))
        }
    };

    let baseline_bytes =
        fs::read(&generic_baseline_json).map_err(|e| (format!("error: {e}\n"), 2))?;

    let mut scorers: HashMap<String, SequentialImportBpeScorer> = HashMap::new();
    let mut filter_adapters: HashMap<String, Box<dyn LanguageAdapter>> = HashMap::new();

    for (lang, lang_cfg) in languages {
        let lc = match lang_cfg.as_object() {
            Some(o) => o,
            None => {
                return Err((
                    format!(
                        "error: {} has malformed entry for language '{}'\n",
                        config_json.display(),
                        lang
                    ),
                    2,
                ))
            }
        };

        let threshold = match lc.get("threshold").and_then(Value::as_f64) {
            Some(t) => t,
            None => {
                return Err((
                    format!("error: failed to load scorer for '{lang}': 'threshold'\n"),
                    2,
                ))
            }
        };

        let get_f64 = |k: &str, d: f64| lc.get(k).and_then(Value::as_f64).unwrap_or(d);
        let get_usize = |k: &str, d: usize| {
            lc.get(k)
                .and_then(Value::as_u64)
                .map(|x| x as usize)
                .unwrap_or(d)
        };
        let get_strings = |k: &str| -> Vec<String> {
            lc.get(k)
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        };

        let cfg = SequentialConfig {
            bpe_threshold: threshold,
            enable_typicality: true,
            exclude_data_dominant: true,
            call_receiver_alpha: get_f64("call_receiver_alpha", 2.0),
            call_receiver_cap: get_usize("call_receiver_cap", 5),
            call_receiver_root_bonus: get_f64("call_receiver_root_bonus", 2.0),
            call_receiver_n_clusters: get_usize("call_receiver_n_clusters", 8),
            call_receiver_cluster_seed: lc
                .get("call_receiver_cluster_seed")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            call_receiver_cluster_bonus: get_f64("call_receiver_cluster_bonus", 5.0),
            call_receiver_cluster_rare_threshold: get_usize(
                "call_receiver_cluster_rare_threshold",
                0,
            ),
            call_receiver_cluster_size_min: get_usize("call_receiver_cluster_size_min", 0),
            call_receiver_rarity_weighting: crate::scoring::call_receiver::RarityWeighting::Off,
            call_receiver_shape_primitive_names: Vec::new(),
            // Real diff hunks routinely start/end mid-construct, so the host
            // fallback is what lets the call-receiver see check-time hunks at
            // all; the calibration side always applied it (symmetry).
            call_receiver_parse_error_host_fallback: lc
                .get("call_receiver_parse_error_host_fallback")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            // from_model reads the fitted convention model (with calibrated
            // bars) from the artifact itself.
            conventions: None,
            convention_bonus: get_f64("convention_bonus", 5.0),
            import_modules: get_strings("import_modules"),
            import_module_prefixes: get_strings("import_module_prefixes"),
            // Parse the optional `evidence_corpus` block. Unlike the Python
            // loader (which requires it), the Rust port keeps evidence optional:
            // a config without the block simply renders no `↳` evidence lines,
            // so the pre-evidence check goldens stay byte-identical.
            evidence_corpus: lc
                .get("evidence_corpus")
                .and_then(EvidenceCorpus::from_json),
            detect: detect.clone(),
        };

        let adapter = match adapter_for_language(lang) {
            Some(a) => a,
            None => {
                return Err((
                    format!(
                        "error: failed to load scorer for '{lang}': Unknown language: '{lang}'\n"
                    ),
                    2,
                ))
            }
        };
        let filter_adapter = adapter_for_language(lang).expect("adapter already built above");

        let model: LanguageModel = match lc.get("model") {
            Some(m) => serde_json::from_value(m.clone()).map_err(|e| {
                (
                    format!("error: failed to load scorer for '{lang}': model: {e}\n"),
                    2,
                )
            })?,
            None => {
                return Err((
                    format!(
                        "error: {} has no 'model' block for language '{}' — regenerate via `argot fit`.\n",
                        config_json.display(),
                        lang
                    ),
                    2,
                ))
            }
        };

        let scorer = SequentialImportBpeScorer::from_model(&model, &baseline_bytes, adapter, cfg)
            .map_err(|e| {
            (
                format!("error: failed to load scorer for '{lang}': {e}\n"),
                2,
            )
        })?;
        scorers.insert(lang.clone(), scorer);
        filter_adapters.insert(lang.clone(), filter_adapter);
    }

    let mut language_extensions: HashSet<String> = HashSet::new();
    for (ext, lang) in EXT_TO_LANG {
        if scorers.contains_key(*lang) {
            language_extensions.insert((*ext).to_string());
        }
    }

    let fit_sha = languages
        .values()
        .filter_map(|lc| lc.get("calibration"))
        .filter_map(|c| c.get("repo_sha"))
        .filter_map(Value::as_str)
        .find(|s| !s.is_empty() && *s != "unknown")
        .map(String::from);

    // Combine the per-language model fingerprints into one overall hash, the
    // same way the manifest does, so `check` can name the model it scored with.
    let per_lang_model_hash: std::collections::BTreeMap<String, String> = languages
        .iter()
        .filter_map(|(lang, lc)| {
            lc.get("model_hash")
                .and_then(Value::as_str)
                .map(|h| (lang.clone(), h.to_string()))
        })
        .collect();
    let model_hash = crate::scoring::calibration::combined_model_hash(&per_lang_model_hash);

    // Per-language slice thresholds (absent for an unsliced fit).
    let mut slices: HashMap<String, Vec<SliceEntry>> = HashMap::new();
    for (lang, lc) in languages {
        let Some(arr) = lc.get("slices").and_then(Value::as_array) else {
            continue;
        };
        let entries: Vec<SliceEntry> = arr
            .iter()
            .filter_map(|s| {
                let threshold = s.get("threshold").and_then(Value::as_f64)?;
                let paths = s
                    .get("paths")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                Some(SliceEntry { paths, threshold })
            })
            .collect();
        if !entries.is_empty() {
            slices.insert(lang.clone(), entries);
        }
    }

    // Per-language new-file thresholds (absent for configs predating the field).
    let mut new_file_thresholds: HashMap<String, f64> = HashMap::new();
    for (lang, lc) in languages {
        if let Some(t) = lc.get("new_file_threshold").and_then(Value::as_f64) {
            new_file_thresholds.insert(lang.clone(), t);
        }
    }

    // Authoritative fit-corpus file set (repo-relative), including data-dominant
    // files (absent for configs predating the field).
    let fit_corpus_files: HashSet<String> = config
        .get("corpus_files")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    Ok(Loaded {
        scorers,
        filter_adapters,
        language_extensions,
        fit_sha,
        model_hash,
        slices,
        new_file_thresholds,
        fit_corpus_files,
    })
}
/// A repo's fitted per-language scorers, loaded once for reuse outside the
/// `check` diff flow (the MCP server scores agent-supplied hunks the same way
/// `check` scores diff hunks — against the frozen fit-time model, never the
/// live tree).
pub struct RepoScorers {
    scorers: HashMap<String, SequentialImportBpeScorer>,
    /// Combined model fingerprint — the `model:` hash `check` reports.
    pub model_hash: String,
}

impl RepoScorers {
    /// Load from a repo's `.argot/`. `detect` is the repo's `[detect]` config
    /// (governs the check-time auto-generated skip). The error carries a
    /// human-readable message (e.g. "run `argot fit` first").
    pub fn load(argot_dir: &Path, detect: &DetectConfig) -> std::result::Result<Self, String> {
        let loaded = load_scorers(argot_dir, detect).map_err(|(msg, _)| msg)?;
        Ok(RepoScorers {
            scorers: loaded.scorers,
            model_hash: loaded.model_hash,
        })
    }

    /// The scoring language name (`"python"`/`"typescript"`) for a file path, or
    /// `None` when the extension isn't supported.
    pub fn language_for(&self, file_path: &str) -> Option<&'static str> {
        ext_to_lang(&extension(file_path))
    }

    /// Score one hunk against the model for its file's language. `None` when the
    /// file's language has no fitted scorer.
    pub fn score(
        &mut self,
        file_path: &str,
        hunk_content: &str,
        file_source: Option<&str>,
    ) -> Option<ScoredHunk> {
        let lang = self.language_for(file_path)?;
        let scorer = self.scorers.get_mut(lang)?;
        Some(scorer.score_hunk(
            hunk_content,
            file_source,
            None,
            None,
            Some(Path::new(file_path)),
        ))
    }
}
