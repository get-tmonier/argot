//! `check` — port of `engine/argot/check.py`.
//!
//! Loads the `.argot/` artifacts (v2 `scorer-config.json`, `repo-corpus.txt`,
//! `generic-baseline.json`), collects git patches for the requested mode
//! (commit / range / workdir / staged / unstaged), scores each hunk through the
//! per-language `SequentialImportBpeScorer`, and renders a decision.
//!
//! This is a behaviour-preserving port: the rendered stdout is byte-identical
//! to the Python engine's (in the `NO_COLOR` / non-tty path), including the
//! per-reason `↳` evidence lines and the eslint-style `^^^^` caret underlines
//! when the config carries an `evidence_corpus` block. On a color-capable tty
//! the human render adds per-severity ANSI accents (red/yellow/blue on the
//! glyph + tier, dim on secondary detail); syntax highlighting of hunk bodies
//! remains deferred.

use crate::config::{ArgotConfig, DetectConfig};
use crate::detector::{run_detectors, CheckContext, Detector, RegisteredDetector, ScanReport};
use crate::finding::{Finding, RenderEvidence, SuppressedBy};
use crate::git_walk::{
    open_repo, resolve_shas, walk_commits, HunkSpan, WalkItem, SUPPORTED_EXTENSIONS,
};
use crate::output::{
    render_github, render_json, render_sarif, FileScan, HitRecord, OutputFormat, ReportMeta,
};
use crate::rules::{self, RuleSettings, RulesLayer, Severity as RuleSeverity};
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
use crate::scoring::evidence::types::{EvidenceCorpus, SourceSpan};
use crate::scoring::model::LanguageModel;
use crate::scoring::sequential::{ScoredHunk, SequentialConfig, SequentialImportBpeScorer};
use crate::suppress::{
    fnmatch, hit_hash, write_last_check, FileSuppressions, LastCheckHit, PathScope,
    PathSuppressions, SuppressionRule,
};
use crate::text::splitlines;
use git2::{DiffFindOptions, Patch, Status, StatusOptions};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

/// Default number of hunk-body lines shown under each above-threshold hit.
pub const DEFAULT_HUNK_LINES: usize = 6;

/// Confidence tier ordering, weakest first. Confidence grades how strong the
/// evidence is (`unusual` / `suspicious` / `foreign`); it is display-only —
/// whether a finding fails the check is decided by its rule's configured
/// severity (`error` / `warn`), never by the tier.
const CONFIDENCE_ORDER: [&str; 3] = ["unusual", "suspicious", "foreign"];

// ANSI color codes for the human `check` render. Every colored write goes
// through `paint`, which is a no-op when `use_color` is false — so the
// `NO_COLOR` / non-tty path stays byte-identical to the parity fixtures.
const C_RED: &str = "\x1b[31m";
const C_YELLOW: &str = "\x1b[33m";
const C_BLUE: &str = "\x1b[34m";
const C_BOLD: &str = "\x1b[1m";
const C_DIM: &str = "\x1b[2m";
const C_RESET: &str = "\x1b[0m";

/// The accent color for a confidence tier: red (foreign), yellow (suspicious),
/// blue (unusual).
fn confidence_color(tier: &str) -> &'static str {
    match tier {
        "foreign" => C_RED,
        "suspicious" => C_YELLOW,
        _ => C_BLUE,
    }
}

/// Wrap `text` in an ANSI code when `use_color`, else return it unchanged.
fn paint(text: &str, color: &str, use_color: bool) -> String {
    if use_color {
        format!("{color}{text}{C_RESET}")
    } else {
        text.to_string()
    }
}

/// Parsed CLI options for `check` (the CLI layer supplies `use_color`).
pub struct CheckArgs {
    pub repo_path: String,
    pub reference: String,
    pub staged: bool,
    pub unstaged: bool,
    pub commit: Option<String>,
    pub only: Vec<String>,
    pub exclude: Vec<String>,
    pub threshold: Option<f64>,
    pub argot_dir: PathBuf,
    pub hunk_lines: usize,
    pub verbose: bool,
    /// Only show hits at or above this confidence tier (display filter).
    pub min_confidence: String,
    /// Validated CLI `--rule` overrides, highest-precedence severity layer.
    pub rule_overrides: RulesLayer,
    /// Promote `warn`-severity findings to check failures (CI strictness).
    pub error_on_warnings: bool,
    /// Insert an inline ignore comment above every current finding (adoption
    /// on an existing codebase — the `ruff --add-noqa` move). Working-tree
    /// modes only.
    pub add_ignores: bool,
    pub use_color: bool,
    /// Output format. Machine formats (`json`/`sarif`) own stdout exclusively.
    pub format: OutputFormat,
    /// Today's date (`YYYY-MM-DD`) for suppression expiry. Core logic never
    /// calls system time — the CLI passes the real date, tests pass fixed ones.
    pub today: String,
}

/// Result of a `check` run — the CLI prints these and exits with `exit_code`.
pub struct CheckOutcome {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CheckOutcome {
    fn err(stderr: String, code: i32) -> Self {
        CheckOutcome {
            stdout: String::new(),
            stderr,
            exit_code: code,
        }
    }
}

/// One file's diff in a single source (`_PatchBatch`). `source` is
/// `workdir`/`staged`/`untracked` for working-tree origins, or a 7-char commit
/// SHA for committed changes.
pub(crate) struct PatchBatch {
    file_path: String,
    content: Vec<u8>,
    hunks: Vec<HunkSpan>,
    source: String,
    /// The file matched a user `[exclude].paths` pattern: still scored (so the
    /// suppression is countable), but every hit is dropped from output and
    /// exit-code consideration.
    ignored_by_pattern: bool,
}

/// The nearest-existing-code evidence attached to a semantic finding (F4). Held
/// as structured data so every output format renders it its own way.
#[cfg(feature = "semantic")]
#[derive(Debug, Clone)]
enum SemanticHitEvidence {
    /// F1 reinvention: the existing function this one duplicates.
    Redundant {
        nearest_symbol: String,
        nearest_path: String,
        nearest_line: usize,
        similarity: f32,
    },
    /// F2 placement: the area this function looks like it belongs in.
    Misplaced {
        neighbor_area: String,
        actual_area: String,
        /// Nearest peers (symbol, path:line) that voted for `neighbor_area`.
        peers: Vec<(String, String, usize)>,
    },
}

/// One calibrated slice for check-time dispatch: its threshold applies to hunks
/// whose repo-relative path matches any of `paths`.
struct SliceEntry {
    paths: Vec<String>,
    threshold: f64,
}

/// Loaded per-language scorers plus the filtering machinery.
struct Loaded {
    scorers: HashMap<String, SequentialImportBpeScorer>,
    filter_adapters: HashMap<String, Box<dyn LanguageAdapter>>,
    language_extensions: HashSet<String>,
    /// Per-language slice thresholds (per-subdirectory / per-author voice).
    /// Empty for an unsliced fit.
    slices: HashMap<String, Vec<SliceEntry>>,
    /// Per-language new-file thresholds. A hunk whose file was absent from the
    /// fit corpus is judged against this (higher) bar instead of `threshold`
    /// (issue #92 new-file flooding). Absent for configs predating the field —
    /// then new files keep the single-threshold behaviour.
    new_file_thresholds: HashMap<String, f64>,
    /// Authoritative fit-corpus file set (repo-relative), including data-dominant
    /// files. A path absent here is a new file. Empty for configs predating the
    /// field — then new-file detection falls back to cluster membership, which
    /// misclassifies data-dominant known files (issue #92).
    fit_corpus_files: HashSet<String>,
    /// Repo SHA the model was fitted at (calibration meta), for the
    /// freshness warning. `None` when the config predates the field.
    fit_sha: Option<String>,
    /// Combined fingerprint of the fit-time model — the same `model_hash` the
    /// manifest records. Lets `check` name which model judged the diff.
    model_hash: String,
}

/// Extension → language name (`_EXT_TO_LANG`).
const EXT_TO_LANG: &[(&str, &str)] = &[
    (".py", "python"),
    (".ts", "typescript"),
    (".tsx", "typescript"),
    (".js", "javascript"),
    (".jsx", "javascript"),
    (".go", "go"),
    (".rs", "rust"),
    (".c", "c"),
    (".h", "c"),
    (".java", "java"),
    (".cs", "csharp"),
    (".php", "php"),
    (".cpp", "cpp"),
    (".cc", "cpp"),
    (".hpp", "cpp"),
    (".cxx", "cpp"),
    (".rb", "ruby"),
];

/// The scoring language name for a lowercase file extension (with dot), or
/// `None` when unsupported. Public so out-of-process consumers of `check`'s
/// JSON (the bench, scripts) classify paths the exact way `check` routes them.
pub fn ext_to_lang(ext: &str) -> Option<&'static str> {
    EXT_TO_LANG.iter().find(|(e, _)| *e == ext).map(|(_, l)| *l)
}

/// [`ext_to_lang`], resolving the `.h` C/C++ ambiguity with the repo-level
/// `header_is_cpp` decision (translation-unit majority) so check routes a
/// header to the same model calibrate built it into. All other extensions are
/// unchanged.
pub fn ext_to_lang_ctx(ext: &str, header_is_cpp: bool) -> Option<&'static str> {
    if header_is_cpp && ext == ".h" {
        return Some("cpp");
    }
    ext_to_lang(ext)
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

/// Python `Path(path).suffix.lower()` (`git_walk._extension`).
pub fn extension(path: &str) -> String {
    let name = match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    };
    match name.rfind('.') {
        Some(i) if i > 0 && i < name.len() - 1 => name[i..].to_ascii_lowercase(),
        _ => String::new(),
    }
}

fn is_supported_ext(file_path: &str) -> bool {
    SUPPORTED_EXTENSIONS.contains(&extension(file_path).as_str())
}

fn confidence_index(s: &str) -> usize {
    CONFIDENCE_ORDER.iter().position(|x| *x == s).unwrap_or(0)
}

/// Classify a hit into a confidence tier.
///
/// Confidence expresses the *strength of the evidence that a hunk is foreign*,
/// derived per signal-kind — not one margin rule for every reason:
///
/// * **Categorical foreign signals** are `foreign` by nature. A foreign import
///   is a dependency the repo has never used (0-usage at the fit SHA) — the
///   top-tier signal, and the one argot catches most reliably. Its score is a
///   *count* of never-before-seen modules against a threshold of 1.0, so the
///   additive margins below (calibrated for the BPE nat scale) would misfile a
///   lone foreign import as `unusual` — the weakest tier — even though it *is*
///   the definition of `foreign`.
/// * **Distributional signals** (BPE surprise, convention rarity, unfamiliar
///   callee) grade by margin above the calibrated threshold: the margin there
///   genuinely measures how far outside the repo's voice the hunk sits.
/// * **Structural findings** (`redundant` / `misplaced` / `layering`) pin to
///   `unusual` — they surface real, linter-invisible structure (a duplicate, a
///   misplacement, a crossed boundary) for the author to judge; their scores
///   are not on the foreignness scale the margins above grade.
/// * **Integrity findings** (`test-deleted` / `test-disabled` /
///   `test-weakened`) pin to `suspicious`: each is a discrete, evidenced
///   event (a marker added, assertions excised) that survived the FP
///   refinements and the repo's own calibrated gates — stronger than
///   `unusual`, but not the categorical certainty of a 0-usage import.
///
/// Whether a finding fails the check is its rule's configured severity
/// (`error` / `warn` / `off`), not this tier.
fn confidence(reason: &str, score: f64, threshold: f64) -> &'static str {
    match reason {
        "import" => "foreign",
        "redundant" | "misplaced" | "layering" => "unusual",
        "test_deleted" | "test_disabled" | "test_weakened" => "suspicious",
        _ => {
            if score >= threshold + 1.5 {
                "foreign"
            } else if score >= threshold + 0.5 {
                "suspicious"
            } else {
                "unusual"
            }
        }
    }
}

/// Scope decision for one patch batch, against the resolved path-suppression
/// set (recommended built-ins + `argot.toml [exclude].paths` — the same set calibration
/// samples from; lock-step principle).
enum BatchScope {
    /// In scope: score and report normally.
    Score,
    /// In scope but matched by a user `[exclude].paths` pattern: score it so the
    /// suppression is countable, then drop its hits from output.
    ScoreSuppressed,
    /// Out of scope (wrong language, recommended exclusion, data-dominant):
    /// silently dropped, exactly as before suppressions existed.
    Drop,
}

/// Languages present in the change that argot supports but the current fit
/// has no model for (fitted before the language appeared in the repo).
fn patches_langs_without_model(
    patches: &[PatchBatch],
    scorers: &HashMap<String, SequentialImportBpeScorer>,
) -> Vec<&'static str> {
    patches
        .iter()
        .filter_map(|b| ext_to_lang(&extension(&b.file_path)))
        .filter(|lang| !scorers.contains_key(*lang))
        .collect()
}

/// Port of `_is_out_of_scope`, split so user-ignored files stay countable:
/// wrong language / recommended-set path → `Drop` (silent, as always); user
/// `[exclude].paths` match → `ScoreSuppressed`. Data-heavy files are NOT dropped
/// here: data scope is row-granular inside the scorer (a planted code hunk in
/// a data-dominant file must still be judged; its data-row hunks are skipped
/// per hunk).
fn batch_scope(
    file_path: &str,
    language_extensions: &HashSet<String>,
    path_suppressions: &PathSuppressions,
) -> BatchScope {
    let ext = extension(file_path);
    if !language_extensions.contains(&ext) {
        return BatchScope::Drop;
    }
    match path_suppressions.classify(file_path) {
        PathScope::Recommended => BatchScope::Drop,
        PathScope::UserIgnored => BatchScope::ScoreSuppressed,
        PathScope::InScope => BatchScope::Score,
    }
}

/// `--exclude` overrides `--only`; empty `only` means "no restriction"
/// (`_apply_filters`).
fn passes_filters(fp: &str, only: &[String], exclude: &[String]) -> bool {
    if exclude.iter().any(|pat| fnmatch(fp, pat)) {
        return false;
    }
    if !only.is_empty() && !only.iter().any(|pat| fnmatch(fp, pat)) {
        return false;
    }
    true
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
fn load_scorers(argot_dir: &Path, detect: &DetectConfig) -> Result<Loaded, (String, i32)> {
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

/// The slice threshold that applies to `rel_path` for `lang`, if any (first
/// matching slice wins — most-specific specs should be listed first at fit).
fn slice_threshold(
    slices: &HashMap<String, Vec<SliceEntry>>,
    lang: &str,
    rel_path: &str,
) -> Option<f64> {
    slices.get(lang)?.iter().find_map(|s| {
        if s.paths
            .iter()
            .any(|p| rel_path == p || rel_path.starts_with(p))
        {
            Some(s.threshold)
        } else {
            None
        }
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

/// Yield batches for committed changes (`_committed_patches`), source = 7-char SHA.
fn committed_patches(repo_path: &str, shas: &HashSet<String>) -> anyhow::Result<Vec<PatchBatch>> {
    let mut out = Vec::new();
    walk_commits(repo_path, shas, |item: WalkItem| {
        let short: String = item.commit_id.chars().take(7).collect();
        out.push(PatchBatch {
            file_path: item.file_path,
            content: item.post_blob,
            hunks: item.hunks,
            source: short,
            ignored_by_pattern: false,
        });
        Ok(ControlFlow::Continue(()))
    })?;
    Ok(out)
}

/// Net diff of a `base..head` range, scored as one changeset — the changes
/// `head` introduces relative to `base` (merge-base → head, matching a pull
/// request's diff), *not* each intervening commit. So when a later commit in the
/// range reverts or rewrites an earlier one (e.g. a fix that drops a foreign
/// import a prior commit added), the range shows only the net result — a fix
/// commit clears the flag, exactly as a reviewer reading the PR's files would
/// expect. Content is the file as `head` leaves it; source = head's short SHA.
fn net_range_patches(
    repo_path: &str,
    base_ref: &str,
    head_ref: &str,
) -> anyhow::Result<Vec<PatchBatch>> {
    let repo = open_repo(repo_path)?;
    let base_commit = repo.revparse_single(base_ref)?.peel_to_commit()?;
    let head_commit = repo.revparse_single(head_ref)?.peel_to_commit()?;
    // Merge-base → head is what `head` adds since diverging from `base`, so a
    // base that advanced past the branch point doesn't show as spurious changes.
    let base_id = repo
        .merge_base(base_commit.id(), head_commit.id())
        .unwrap_or_else(|_| base_commit.id());
    let base_tree = repo.find_commit(base_id)?.tree()?;
    let head_tree = head_commit.tree()?;
    let mut diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;
    diff.find_similar(Some(&mut DiffFindOptions::new()))?;
    let short: String = head_commit.id().to_string().chars().take(7).collect();
    let mut out = Vec::new();
    for idx in 0..diff.deltas().len() {
        let delta = match diff.get_delta(idx) {
            Some(d) => d,
            None => continue,
        };
        let file_path = match delta.new_file().path().and_then(|p| p.to_str()) {
            Some(p) => p.to_string(),
            None => continue,
        };
        if !is_supported_ext(&file_path) {
            continue;
        }
        let patch = match Patch::from_diff(&diff, idx)? {
            Some(p) => p,
            None => continue,
        };
        if patch.num_hunks() == 0 {
            continue;
        }
        let hunks = hunks_from_patch(&patch)?;
        // Post-state content: the file as it stands at `head` (deleted → skip).
        let content = match head_tree
            .get_path(Path::new(&file_path))
            .ok()
            .and_then(|e| repo.find_blob(e.id()).ok())
        {
            Some(b) => b.content().to_vec(),
            None => continue,
        };
        out.push(PatchBatch {
            file_path,
            content,
            hunks,
            source: short.clone(),
            ignored_by_pattern: false,
        });
    }
    Ok(out)
}

fn hunks_from_patch(patch: &Patch) -> anyhow::Result<Vec<HunkSpan>> {
    let n = patch.num_hunks();
    let mut hunks = Vec::with_capacity(n);
    for h in 0..n {
        let (hunk, _lines) = patch.hunk(h)?;
        hunks.push(HunkSpan {
            new_start: hunk.new_start(),
            new_lines: hunk.new_lines(),
        });
    }
    Ok(hunks)
}

/// Unstaged changes vs the index (`_modified_patches`, source="workdir").
fn modified_patches(repo_path: &str) -> anyhow::Result<Vec<PatchBatch>> {
    let repo = open_repo(repo_path)?;
    let index = repo.index()?;
    let mut diff = match repo.diff_index_to_workdir(Some(&index), None) {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };
    diff.find_similar(Some(&mut DiffFindOptions::new()))?;
    let workdir = match repo.workdir() {
        Some(w) => w.to_path_buf(),
        None => return Ok(Vec::new()),
    };
    let mut out = Vec::new();
    for idx in 0..diff.deltas().len() {
        let delta = match diff.get_delta(idx) {
            Some(d) => d,
            None => continue,
        };
        let file_path = match delta.new_file().path().and_then(|p| p.to_str()) {
            Some(p) => p.to_string(),
            None => continue,
        };
        if !is_supported_ext(&file_path) {
            continue;
        }
        let patch = match Patch::from_diff(&diff, idx)? {
            Some(p) => p,
            None => continue,
        };
        if patch.num_hunks() == 0 {
            continue;
        }
        let hunks = hunks_from_patch(&patch)?;
        let full = workdir.join(&file_path);
        if !full.exists() {
            continue;
        }
        let content = fs::read(&full)?;
        out.push(PatchBatch {
            file_path,
            content,
            hunks,
            source: "workdir".to_string(),
            ignored_by_pattern: false,
        });
    }
    Ok(out)
}

/// Staged changes vs HEAD (`_staged_patches`, source="staged"). Content from
/// the index blob.
fn staged_patches(repo_path: &str) -> anyhow::Result<Vec<PatchBatch>> {
    let repo = open_repo(repo_path)?;
    let index = repo.index()?;
    let head_tree = match repo.head().and_then(|h| h.peel_to_tree()) {
        Ok(t) => t,
        Err(_) => return Ok(Vec::new()),
    };
    let mut diff = match repo.diff_tree_to_index(Some(&head_tree), Some(&index), None) {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };
    diff.find_similar(Some(&mut DiffFindOptions::new()))?;
    let mut out = Vec::new();
    for idx in 0..diff.deltas().len() {
        let delta = match diff.get_delta(idx) {
            Some(d) => d,
            None => continue,
        };
        let file_path = match delta.new_file().path().and_then(|p| p.to_str()) {
            Some(p) => p.to_string(),
            None => continue,
        };
        if !is_supported_ext(&file_path) {
            continue;
        }
        let patch = match Patch::from_diff(&diff, idx)? {
            Some(p) => p,
            None => continue,
        };
        if patch.num_hunks() == 0 {
            continue;
        }
        let hunks = hunks_from_patch(&patch)?;
        let entry = match index.get_path(Path::new(&file_path), 0) {
            Some(e) => e,
            None => continue,
        };
        let blob = match repo.find_blob(entry.id) {
            Ok(b) => b,
            Err(_) => continue,
        };
        out.push(PatchBatch {
            file_path,
            content: blob.content().to_vec(),
            hunks,
            source: "staged".to_string(),
            ignored_by_pattern: false,
        });
    }
    Ok(out)
}

/// Untracked supported files (`_untracked_patches`, source="untracked"). One
/// synthetic full-file hunk each.
fn untracked_patches(repo_path: &str) -> anyhow::Result<Vec<PatchBatch>> {
    let repo = open_repo(repo_path)?;
    let workdir = match repo.workdir() {
        Some(w) => w.to_path_buf(),
        None => return Ok(Vec::new()),
    };
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut opts))?;
    let mut out = Vec::new();
    for entry in statuses.iter() {
        if !entry.status().contains(Status::WT_NEW) {
            continue;
        }
        let file_path = match entry.path() {
            Some(p) => p.to_string(),
            None => continue,
        };
        if !is_supported_ext(&file_path) {
            continue;
        }
        let full = workdir.join(&file_path);
        if !full.exists() {
            continue;
        }
        let content = fs::read(&full)?;
        let source = String::from_utf8_lossy(&content);
        let line_count = splitlines(&source).len();
        if line_count == 0 {
            continue;
        }
        out.push(PatchBatch {
            file_path,
            content,
            hunks: vec![HunkSpan {
                new_start: 1,
                new_lines: line_count as u32,
            }],
            source: "untracked".to_string(),
            ignored_by_pattern: false,
        });
    }
    Ok(out)
}

fn chain_workdir_patches(repo_path: &str) -> anyhow::Result<Vec<PatchBatch>> {
    let mut out = modified_patches(repo_path)?;
    out.extend(staged_patches(repo_path)?);
    out.extend(untracked_patches(repo_path)?);
    Ok(out)
}

/// Score each hunk, dispatching per language (`_score_patches`). Applies the
/// inline-comment and `[[mute]]` surfaces per hit (path-level `[exclude].paths`
/// suppression arrives pre-marked on the batch). Returns
/// `(hits, hunk_count, per-file hunk counts)`.
#[allow(clippy::too_many_arguments)]
fn score_patches(
    patches: &[PatchBatch],
    scorers: &mut HashMap<String, SequentialImportBpeScorer>,
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    slices: &HashMap<String, Vec<SliceEntry>>,
    new_file_thresholds: &HashMap<String, f64>,
    fit_corpus_files: &HashSet<String>,
    mute_rules: &[SuppressionRule],
    header_cpp: bool,
    stderr: &mut String,
) -> (Vec<Finding>, usize, Vec<FileScan>) {
    let mut hits: Vec<Finding> = Vec::new();
    let mut hunk_count = 0usize;
    let mut file_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut warned: HashSet<String> = HashSet::new();
    // Per-changeset novel-import dedup: foreign top-level modules that have
    // already raised an import alert in this check run. The same new dependency
    // added across many files of one change (a mechanical migration) is one
    // decision — alert on its first appearance, dedup the rest.
    let mut alerted_foreign_modules: HashSet<String> = HashSet::new();
    let mut deduped_import_alerts: usize = 0;

    for batch in patches {
        let ext = extension(&batch.file_path);
        let scorer = match ext_to_lang_ctx(&ext, header_cpp).and_then(|l| scorers.get_mut(l)) {
            Some(s) => s,
            None => {
                stderr.push_str(&format!(
                    "[argot] skipping {}: no scorer for extension '{}'\n",
                    batch.file_path, ext
                ));
                continue;
            }
        };

        let file_source = String::from_utf8_lossy(&batch.content).into_owned();
        let file_lines = splitlines(&file_source);
        let n_lines = file_lines.len() as i64;

        // The file's suppression surfaces, resolved once from the same content
        // that gets scored (inline comments use the language's comment token).
        let suppressions = FileSuppressions::parse(
            &batch.file_path,
            &file_source,
            ext_to_lang(&ext)
                .and_then(|l| filter_adapters.get(l))
                .map(|a| a.line_comment_prefix()),
            mute_rules,
            batch.ignored_by_pattern,
        );
        for w in suppressions.warnings() {
            let msg = format!("[argot] {}:{}: {}\n", batch.file_path, w.line, w.message);
            if warned.insert(msg.clone()) {
                stderr.push_str(&msg);
            }
        }

        for hunk in &batch.hunks {
            hunk_count += 1;
            *file_counts.entry(batch.file_path.clone()).or_insert(0) += 1;
            let hunk_start = hunk.new_start as i64 - 1;
            let hunk_end = hunk_start + hunk.new_lines as i64;
            if hunk_start < 0 || hunk_end > n_lines {
                continue;
            }
            let hs = hunk_start as usize;
            let he = hunk_end as usize;
            let hunk_content = file_lines[hs..he].join("\n");
            // file_path routes the hunk to its fit-time cluster (falling back
            // to Jaccard-nearest for files the model has never seen) — the
            // same signal surface calibration hunks scored against, so the
            // threshold and the check path see one score distribution.
            let scored = scorer.score_hunk(
                &hunk_content,
                Some(&file_source),
                Some(hs + 1),
                Some(he),
                Some(Path::new(&batch.file_path)),
            );
            let line = hunk.new_start as usize;
            let line_end = (hunk.new_start + hunk.new_lines).saturating_sub(1) as usize;
            let reason = scored.reason.as_str().to_string();
            let lang = ext_to_lang(&ext);
            // New-file dispatch takes precedence: a hunk whose file was absent
            // from the fit corpus is judged against the (higher) new-file
            // threshold — a new file gets full unattested-callee mass with no
            // cluster routing, a systematically higher distribution than an edit
            // to a known file (issue #92 new-file flooding). Foreign imports
            // still fire regardless of threshold. Falls through to per-slice /
            // whole-repo dispatch for known files, or configs without the field.
            let is_new_file = if fit_corpus_files.is_empty() {
                // Config predates the corpus_files snapshot: fall back to cluster
                // membership (misclassifies data-dominant known files).
                !scorer.is_fit_file(Path::new(&batch.file_path))
            } else {
                !fit_corpus_files.contains(&batch.file_path)
            };
            let new_file_threshold = lang.and_then(|l| {
                is_new_file
                    .then(|| new_file_thresholds.get(l).copied())
                    .flatten()
            });
            // A `none`-reason hunk fired no stage: its call-receiver
            // contribution was *not* gated (the hunk reaches nothing foreign),
            // so it must not count toward the new-file / slice threshold —
            // otherwise a new file of the repo's own code (its own unattested
            // callees) is flagged on exactly the signal the hunk-level
            // foreign-reach gate already rejected. Judge it on token surprise
            // alone. Firing reasons (import/bpe/call_receiver) already carry a
            // gated score in `scored.score`.
            let new_score = if reason == "none" {
                scored.stages.bpe_score
            } else {
                scored.score
            };
            let (mut flagged, threshold) = match new_file_threshold {
                Some(t) => (reason == "import" || new_score >= t, t),
                None => match lang.and_then(|l| slice_threshold(slices, l, &batch.file_path)) {
                    Some(t) => (reason == "import" || new_score >= t, t),
                    None => (scored.flagged, scored.threshold),
                },
            };
            // Per-changeset novel-import dedup: an import alert whose foreign
            // modules were all already alerted in this run is the same decision
            // seen again (one dependency spread across a migration). Alert on
            // the first appearance; dedup the repeats. A hunk that adds a
            // genuinely new foreign module still fires.
            if flagged && reason == "import" && !scored.foreign_import_modules.is_empty() {
                if scored
                    .foreign_import_modules
                    .iter()
                    .all(|m| alerted_foreign_modules.contains(m))
                {
                    flagged = false;
                    deduped_import_alerts += 1;
                } else {
                    alerted_foreign_modules.extend(scored.foreign_import_modules.iter().cloned());
                }
            }
            let hash = hit_hash(&batch.file_path, &reason, &hunk_content);
            let suppressed_by = suppressions.classify(&reason, &hash, line, line_end);
            hits.push(Finding {
                score: scored.score,
                file_path: batch.file_path.clone(),
                line,
                line_end,
                source: batch.source.clone(),
                reason,
                flagged,
                threshold,
                hunk_content,
                evidence: scored
                    .evidence
                    .map(|e| Box::new(e) as Box<dyn RenderEvidence>),
                hash,
                suppressed_by,
            });
        }
    }
    if deduped_import_alerts > 0 {
        stderr.push_str(&format!(
            "[argot] {deduped_import_alerts} repeat novel-import alert(s) deduped \
             (same dependency across the change)\n"
        ));
    }

    let files_scanned = file_counts
        .into_iter()
        .map(|(path, hunks)| FileScan { path, hunks })
        .collect();
    (hits, hunk_count, files_scanned)
}

/// The base statistical pass (the voice group) as a detector. Borrows the
/// loaded per-language model state; the only detector that fills
/// [`ScanReport`].
struct VoiceDetector<'a> {
    scorers: &'a mut HashMap<String, SequentialImportBpeScorer>,
    slices: &'a HashMap<String, Vec<SliceEntry>>,
    new_file_thresholds: &'a HashMap<String, f64>,
    fit_corpus_files: &'a HashSet<String>,
}

impl Detector for VoiceDetector<'_> {
    fn group(&self) -> &'static str {
        rules::GROUP_VOICE
    }

    fn timing_label(&self) -> &'static str {
        "check: score patches (statistical)"
    }

    /// Always runs: it owns the scan statistics (hunk/file counts in the
    /// report meta), and internal reasons (`none` under `--threshold`) have
    /// no rule to gate on. Off-rule findings are dropped by the engine.
    fn enabled(&self, _settings: &RuleSettings) -> bool {
        true
    }

    fn check(&mut self, ctx: &mut CheckContext<'_>) -> Vec<Finding> {
        let (hits, hunk_count, files_scanned) = score_patches(
            ctx.batches,
            self.scorers,
            ctx.filter_adapters,
            self.slices,
            self.new_file_thresholds,
            self.fit_corpus_files,
            ctx.mute_rules,
            ctx.header_cpp,
            ctx.stderr,
        );
        ctx.scan.hunk_count = hunk_count;
        ctx.scan.files_scanned = files_scanned;
        hits
    }
}

/// The architecture-graph pass — additive `Finding`s from the per-repo
/// module-dependency graph (`.argot/layering.json`). For each changed file it
/// takes the ADDED lines, resolves the internal import edges they introduce, and
/// flags any that reverse an established layer direction or leave a (near-)sink —
/// a boundary the repo never crosses. Runs alongside the statistical scorers,
/// never through them; empty (graceful degrade) when the graph is absent, so the
/// base guardrail is entirely unaffected. Reason code `layering`.
#[cfg(feature = "arch")]
fn arch_hits(
    patches: &[PatchBatch],
    argot_dir: &Path,
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    mute_rules: &[SuppressionRule],
    stderr: &mut String,
) -> Vec<Finding> {
    use crate::scoring::arch_graph::{RepoLayering, LAYERING_FILE};
    let Ok(raw) = std::fs::read_to_string(argot_dir.join(LAYERING_FILE)) else {
        return Vec::new();
    };
    let Some(graph) = RepoLayering::from_json(&raw) else {
        stderr.push_str("[argot] layering graph unreadable\n");
        return Vec::new();
    };
    let mut hits = Vec::new();
    for batch in patches {
        if batch.ignored_by_pattern || !batch.file_path.ends_with(".py") {
            continue; // v1: Python resolver only
        }
        let source = String::from_utf8_lossy(&batch.content);
        let lines: Vec<&str> = source.lines().collect();
        // Concatenate the ADDED lines (1-indexed) — the imports the diff introduces.
        let mut added = String::new();
        let mut first_line = 0usize;
        for h in &batch.hunks {
            for l in h.new_start..(h.new_start + h.new_lines) {
                if let Some(t) = lines.get((l as usize).saturating_sub(1)) {
                    if first_line == 0 {
                        first_line = l as usize;
                    }
                    added.push_str(t);
                    added.push('\n');
                }
            }
        }
        if added.is_empty() {
            continue;
        }
        // Fire if the added imports create a novel reversal/sink-out edge —
        // and keep that edge: the evidence line names the direction it breaks.
        let Some((edge, violation)) = graph
            .file_edges(&batch.file_path, &added)
            .iter()
            .find_map(|e| graph.classify(e).map(|v| (e.clone(), v)))
        else {
            continue;
        };
        let hunk_content = added.clone();
        let hash = hit_hash(&batch.file_path, "layering", &hunk_content);
        let suppressions = FileSuppressions::parse(
            &batch.file_path,
            &source,
            ext_to_lang(&extension(&batch.file_path))
                .and_then(|l| filter_adapters.get(l))
                .map(|a| a.line_comment_prefix()),
            mute_rules,
            false, // ignored-by-pattern batches were skipped above
        );
        let suppressed_by = suppressions.classify("layering", &hash, first_line, first_line);
        hits.push(Finding {
            score: 1.0,
            file_path: batch.file_path.clone(),
            line: first_line,
            line_end: first_line,
            source: batch.source.clone(),
            reason: "layering".to_string(),
            flagged: true,
            threshold: 0.5,
            hunk_content,
            evidence: Some(Box::new(ArchEvidence(arch_evidence(&edge, violation)))),
            hash,
            suppressed_by,
        });
    }
    hits
}

/// The rendered evidence of a `layering` finding — one pre-formatted line
/// naming the established direction the novel edge violates.
#[cfg(feature = "arch")]
struct ArchEvidence(String);

#[cfg(feature = "arch")]
impl RenderEvidence for ArchEvidence {
    fn human(&self, use_color: bool, _hunk_start_line: usize) -> Vec<String> {
        vec![paint(&format!("    ↳ {}", self.0), C_DIM, use_color)]
    }

    fn machine(&self, _hunk_start_line: usize) -> Vec<String> {
        vec![format!("↳ {}", self.0)]
    }
}

/// The architecture group's detection pass.
#[cfg(feature = "arch")]
struct ArchDetector;

#[cfg(feature = "arch")]
impl Detector for ArchDetector {
    fn group(&self) -> &'static str {
        rules::GROUP_ARCHITECTURE
    }

    fn timing_label(&self) -> &'static str {
        "check: arch pass"
    }

    fn check(&mut self, ctx: &mut CheckContext<'_>) -> Vec<Finding> {
        arch_hits(
            ctx.batches,
            &ctx.args.argot_dir,
            ctx.filter_adapters,
            ctx.mute_rules,
            ctx.stderr,
        )
    }
}

/// The evidence line for a `layering` finding: name the established direction
/// the novel edge `(a, b)` breaks, in the repo's own module vocabulary.
#[cfg(feature = "arch")]
fn arch_evidence(
    edge: &crate::scoring::arch_graph::Edge,
    violation: crate::scoring::arch_graph::Violation,
) -> String {
    use crate::scoring::arch_graph::Violation;
    let (a, b) = edge;
    match violation {
        Violation::Reversal => {
            format!("{b} → {a} is this repo's direction — this import reverses it")
        }
        Violation::TransitiveReversal => format!(
            "{b} already depends on {a} — this import closes a cycle against the repo's layering"
        ),
        Violation::SinkOut => {
            format!("{a} is a module this repo never imports out of — this import leaves it")
        }
    }
}

/// Collect the test-integrity pass's changesets: both sides of every changed
/// source file (renames resolved) **including deletions**, which the scoring
/// `PatchBatch` path never carries. Mirrors `collect_patches`' mode dispatch;
/// an explicit commit set yields one changeset per commit so the event
/// refinements reason about each accepted unit separately. Every changeset is
/// labelled with its display source (`workdir` / `staged` / short SHA).
#[cfg(feature = "integrity")]
fn integrity_changesets(
    args: &CheckArgs,
) -> Vec<(String, Vec<crate::scoring::integrity::FileChange>)> {
    use crate::scoring::integrity::FileChange;
    use crate::scoring::test_inventory::language_for_path;

    const MAX_BLOB: usize = 400_000;

    fn tree_text(repo: &git2::Repository, tree: &git2::Tree, path: &str) -> Option<String> {
        let entry = tree.get_path(Path::new(path)).ok()?;
        let blob = repo.find_blob(entry.id()).ok()?;
        (blob.size() <= MAX_BLOB).then(|| String::from_utf8_lossy(blob.content()).to_string())
    }
    fn workdir_text(repo: &git2::Repository, path: &str) -> Option<String> {
        let full = repo.workdir()?.join(path);
        let data = fs::read(&full).ok()?;
        (data.len() <= MAX_BLOB).then(|| String::from_utf8_lossy(&data).to_string())
    }
    fn index_text(repo: &git2::Repository, path: &str) -> Option<String> {
        let index = repo.index().ok()?;
        let entry = index.get_path(Path::new(path), 0)?;
        let blob = repo.find_blob(entry.id).ok()?;
        (blob.size() <= MAX_BLOB).then(|| String::from_utf8_lossy(blob.content()).to_string())
    }
    fn changes_from_diff(
        diff: &mut git2::Diff,
        old_side: &dyn Fn(&str) -> Option<String>,
        new_side: &dyn Fn(&str) -> Option<String>,
    ) -> Vec<FileChange> {
        let _ = diff.find_similar(Some(&mut DiffFindOptions::new()));
        let mut out = Vec::new();
        for d in diff.deltas() {
            let new_path = d
                .new_file()
                .path()
                .and_then(|p| p.to_str())
                .map(str::to_string);
            let old_path = d
                .old_file()
                .path()
                .and_then(|p| p.to_str())
                .map(str::to_string);
            let path = new_path
                .clone()
                .or_else(|| old_path.clone())
                .unwrap_or_default();
            if language_for_path(&path).is_none() {
                continue;
            }
            let old = match d.status() {
                git2::Delta::Added | git2::Delta::Untracked => None,
                _ => old_path.as_deref().and_then(old_side),
            };
            let new = match d.status() {
                git2::Delta::Deleted => None,
                _ => new_path.as_deref().and_then(new_side),
            };
            if old.is_none() && new.is_none() {
                continue;
            }
            out.push(FileChange { path, old, new });
        }
        out
    }
    fn one(source: &str, cs: Vec<FileChange>) -> Vec<(String, Vec<FileChange>)> {
        if cs.is_empty() {
            Vec::new()
        } else {
            vec![(source.to_string(), cs)]
        }
    }

    let repo_path = args.repo_path.as_str();
    let Ok(repo) = open_repo(repo_path) else {
        return Vec::new();
    };
    let commit_set = args
        .commit
        .as_deref()
        .map(|c| !c.is_empty())
        .unwrap_or(false);
    let ref_nonempty = !args.reference.is_empty();

    let per_commit = |shas: &HashSet<String>| -> Vec<(String, Vec<FileChange>)> {
        let mut out = Vec::new();
        for sha in shas {
            let Ok(oid) = git2::Oid::from_str(sha) else {
                continue;
            };
            let Ok(commit) = repo.find_commit(oid) else {
                continue;
            };
            if commit.parent_count() != 1 {
                continue;
            }
            let Ok(parent_tree) = commit.parent(0).and_then(|p| p.tree()) else {
                continue;
            };
            let Ok(tree) = commit.tree() else {
                continue;
            };
            let Ok(mut diff) = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None) else {
                continue;
            };
            let cs = changes_from_diff(&mut diff, &|p| tree_text(&repo, &parent_tree, p), &|p| {
                tree_text(&repo, &tree, p)
            });
            if !cs.is_empty() {
                let short: String = sha.chars().take(7).collect();
                out.push((short, cs));
            }
        }
        out
    };

    if commit_set {
        let Ok(shas) = resolve_shas(&repo, args.commit.as_deref().unwrap_or_default()) else {
            return Vec::new();
        };
        return per_commit(&shas);
    }
    if ref_nonempty {
        let reference = args.reference.as_str();
        if let Some((base_raw, head_raw)) = reference.split_once("..") {
            let base = if base_raw.is_empty() {
                "HEAD"
            } else {
                base_raw
            };
            let head_trimmed = head_raw.trim_start_matches('.');
            let head = if head_trimmed.is_empty() {
                "HEAD"
            } else {
                head_trimmed
            };
            let Ok(base_c) = repo.revparse_single(base).and_then(|o| o.peel_to_commit()) else {
                return Vec::new();
            };
            let Ok(head_c) = repo.revparse_single(head).and_then(|o| o.peel_to_commit()) else {
                return Vec::new();
            };
            let base_id = repo
                .merge_base(base_c.id(), head_c.id())
                .unwrap_or_else(|_| base_c.id());
            let Ok(base_tree) = repo.find_commit(base_id).and_then(|c| c.tree()) else {
                return Vec::new();
            };
            let Ok(head_tree) = head_c.tree() else {
                return Vec::new();
            };
            let Ok(mut diff) = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)
            else {
                return Vec::new();
            };
            let short: String = head_c.id().to_string().chars().take(7).collect();
            let cs = changes_from_diff(&mut diff, &|p| tree_text(&repo, &base_tree, p), &|p| {
                tree_text(&repo, &head_tree, p)
            });
            return one(&short, cs);
        }
        // Bare ref: the net view merge-base(ref, HEAD) → working tree.
        let Ok(base_c) = repo
            .revparse_single(reference)
            .and_then(|o| o.peel_to_commit())
        else {
            return Vec::new();
        };
        let base_id = repo
            .head()
            .ok()
            .and_then(|h| h.target())
            .and_then(|h| repo.merge_base(base_c.id(), h).ok())
            .unwrap_or_else(|| base_c.id());
        let Ok(base_tree) = repo.find_commit(base_id).and_then(|c| c.tree()) else {
            return Vec::new();
        };
        let mut opts = git2::DiffOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(true);
        let Ok(mut diff) = repo.diff_tree_to_workdir_with_index(Some(&base_tree), Some(&mut opts))
        else {
            return Vec::new();
        };
        let cs = changes_from_diff(&mut diff, &|p| tree_text(&repo, &base_tree, p), &|p| {
            workdir_text(&repo, p)
        });
        return one("workdir", cs);
    }
    if args.staged {
        let Ok(head_tree) = repo.head().and_then(|h| h.peel_to_tree()) else {
            return Vec::new();
        };
        let Ok(index) = repo.index() else {
            return Vec::new();
        };
        let Ok(mut diff) = repo.diff_tree_to_index(Some(&head_tree), Some(&index), None) else {
            return Vec::new();
        };
        let cs = changes_from_diff(&mut diff, &|p| tree_text(&repo, &head_tree, p), &|p| {
            index_text(&repo, p)
        });
        return one("staged", cs);
    }
    if args.unstaged {
        let Ok(index) = repo.index() else {
            return Vec::new();
        };
        let Ok(mut diff) = repo.diff_index_to_workdir(Some(&index), None) else {
            return Vec::new();
        };
        let cs = changes_from_diff(&mut diff, &|p| index_text(&repo, p), &|p| {
            workdir_text(&repo, p)
        });
        return one("workdir", cs);
    }
    let Ok(head_tree) = repo.head().and_then(|h| h.peel_to_tree()) else {
        return Vec::new();
    };
    let mut opts = git2::DiffOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    let Ok(mut diff) = repo.diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut opts))
    else {
        return Vec::new();
    };
    let cs = changes_from_diff(&mut diff, &|p| tree_text(&repo, &head_tree, p), &|p| {
        workdir_text(&repo, p)
    });
    one("workdir", cs)
}

/// The rendered evidence of a test-integrity finding — the gamed test and the
/// co-changed production source, plus the affected test's name (`None` for
/// whole-file events) surfaced as `HitRecord.symbol` so consumers can act on
/// the name (e.g. audit attributing a deleted test to the commit whose diff
/// dropped it) without parsing evidence text.
#[cfg(feature = "integrity")]
struct IntegrityEvidence {
    line: String,
    symbol: Option<String>,
}

#[cfg(feature = "integrity")]
impl RenderEvidence for IntegrityEvidence {
    fn human(&self, use_color: bool, _hunk_start_line: usize) -> Vec<String> {
        vec![paint(&format!("    ↳ {}", self.line), C_DIM, use_color)]
    }

    fn machine(&self, _hunk_start_line: usize) -> Vec<String> {
        vec![format!("↳ {}", self.line)]
    }

    fn symbol(&self) -> Option<String> {
        self.symbol.clone()
    }
}

/// The test-integrity pass — additive `Finding`s from diffing both sides of the
/// change's test files into gaming events, gated by the repo's own learned
/// event gates (`.argot/integrity.json`). Runs beside the statistical
/// scorers; a graceful no-op when the changeset carries no tests. Reasons
/// `test_deleted` / `test_disabled` / `test_weakened`.
#[cfg(feature = "integrity")]
fn integrity_hits(
    args: &CheckArgs,
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    mute_rules: &[SuppressionRule],
    stderr: &mut String,
) -> Vec<Finding> {
    use crate::scoring::integrity::{changeset_events, IntegrityModel, INTEGRITY_FILE};

    let model = match std::fs::read_to_string(args.argot_dir.join(INTEGRITY_FILE)) {
        Ok(raw) => match IntegrityModel::from_json(&raw) {
            Some(m) => m,
            None => {
                stderr.push_str(
                    "[argot] integrity gates unreadable — run `argot fit` to restore the test-integrity rules\n",
                );
                return Vec::new();
            }
        },
        // No artifact (an older fit): the built-in default gates apply.
        Err(_) => IntegrityModel::permissive(),
    };

    let mut hits = Vec::new();
    for (source, files) in integrity_changesets(args) {
        for ev in changeset_events(&files) {
            if !model.enabled(ev.kind) {
                continue;
            }
            let reason = ev.kind.reason();
            let hash = hit_hash(&ev.file, reason, &ev.hash_content());
            // Display body: the post-image line the event anchors to (the
            // hash above never depends on it).
            let hunk_content = files
                .iter()
                .find(|f| f.path == ev.file)
                .and_then(|f| f.new.as_deref())
                .and_then(|src| src.lines().nth(ev.line.saturating_sub(1)))
                .unwrap_or_default()
                .to_string();
            let suppressed_by = {
                let new_side = files
                    .iter()
                    .find(|f| f.path == ev.file)
                    .and_then(|f| f.new.as_deref());
                let suppressions = FileSuppressions::parse(
                    &ev.file,
                    new_side.unwrap_or_default(),
                    new_side.and(
                        ext_to_lang(&extension(&ev.file))
                            .and_then(|l| filter_adapters.get(l))
                            .map(|a| a.line_comment_prefix()),
                    ),
                    mute_rules,
                    false,
                );
                suppressions.classify(reason, &hash, ev.line, ev.line)
            };
            hits.push(Finding {
                score: 1.0,
                file_path: ev.file.clone(),
                line: ev.line,
                line_end: ev.line,
                source: source.clone(),
                reason: reason.to_string(),
                flagged: true,
                threshold: 0.5,
                hunk_content,
                evidence: Some(Box::new(IntegrityEvidence {
                    line: ev.evidence(),
                    symbol: (!ev.test_name.is_empty()).then(|| ev.test_name.clone()),
                })),
                hash,
                suppressed_by,
            });
        }
    }
    hits
}

/// The integrity group's detection pass.
#[cfg(feature = "integrity")]
struct IntegrityDetector;

#[cfg(feature = "integrity")]
impl Detector for IntegrityDetector {
    fn group(&self) -> &'static str {
        rules::GROUP_INTEGRITY
    }

    fn timing_label(&self) -> &'static str {
        "check: integrity pass"
    }

    fn check(&mut self, ctx: &mut CheckContext<'_>) -> Vec<Finding> {
        integrity_hits(ctx.args, ctx.filter_adapters, ctx.mute_rules, ctx.stderr)
    }
}

/// The semantic group's detection pass. Skipped whole when both semantic
/// rules are off: no index load, no model download, no cost.
#[cfg(feature = "semantic")]
struct SemanticDetector;

#[cfg(feature = "semantic")]
impl Detector for SemanticDetector {
    fn group(&self) -> &'static str {
        rules::GROUP_SEMANTIC
    }

    fn timing_label(&self) -> &'static str {
        "check: semantic pass"
    }

    fn check(&mut self, ctx: &mut CheckContext<'_>) -> Vec<Finding> {
        semantic_hits(
            ctx.batches,
            &ctx.args.argot_dir,
            ctx.filter_adapters,
            ctx.mute_rules,
            ctx.detect,
            ctx.header_cpp,
            ctx.stderr,
        )
    }
}

/// The semantic pass (F1 reinvention, F2 placement) — additive `Finding`s from
/// the per-repo embedding index. It runs *alongside* the
/// statistical scorers, never through them: it reads `.argot/semantic-index.json`
/// plus the embedder, finds the functions the diff *defines*, and flags any that
/// reinvent existing code. Returns extra hits to merge into the report. Empty
/// (a clean graceful degrade) when the index or model is unavailable, so the
/// base guardrail is entirely unaffected.
#[cfg(feature = "semantic")]
fn semantic_hits(
    patches: &[PatchBatch],
    argot_dir: &Path,
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    mute_rules: &[SuppressionRule],
    detect: &DetectConfig,
    header_cpp: bool,
    stderr: &mut String,
) -> Vec<Finding> {
    use crate::scoring::semantic::embedder::Embedder;
    use crate::scoring::semantic::index::{
        functions_in_file, FunctionRef, LoadedIndex, SemanticArtifact,
    };
    use crate::scoring::semantic::placement::PlacementScorer;
    use crate::scoring::semantic::redundant::RedundantScorer;
    use crate::scoring::semantic::SEMANTIC_INDEX_FILE;

    // Load the fit-time index artifact; its absence just means no semantic layer.
    let t_art = crate::timing::phase("check: semantic artifact read+parse");
    let Ok(raw) = std::fs::read_to_string(argot_dir.join(SEMANTIC_INDEX_FILE)) else {
        return Vec::new();
    };
    let artifact = match SemanticArtifact::from_json_str(&raw) {
        Ok(a) => a,
        Err(e) => {
            stderr.push_str(&format!("[argot] semantic index unreadable: {e}\n"));
            return Vec::new();
        }
    };
    t_art.done();
    // A stale index (older format, different embedding model) must never be
    // queried — its cosines would be silently wrong. Loud skip + rebuild hint.
    if let Err(reason) = artifact.validate_current() {
        stderr.push_str(&format!(
            "[argot] semantic index {reason} — run `argot fit` to rebuild; \
             redundant/misplaced checks skipped this run\n"
        ));
        return Vec::new();
    }

    // Gather the functions this diff defines: a function whose definition line is
    // among the diff's added lines is newly added (its whole body, incl. the def,
    // is in an added hunk) — the reinvention candidates.
    let t_cand = crate::timing::phase("check: semantic candidate extract");
    let mut candidates: Vec<(usize, &'static str, FunctionRef)> = Vec::new();
    for (bi, batch) in patches.iter().enumerate() {
        if batch.ignored_by_pattern {
            continue;
        }
        let ext = extension(&batch.file_path);
        let Some(lang) = ext_to_lang_ctx(&ext, header_cpp) else {
            continue;
        };
        let Some(adapter) = filter_adapters.get(lang) else {
            continue;
        };
        let source = String::from_utf8_lossy(&batch.content);
        // Mirror the index scope (calibration's `filtered`): a data-dominant or
        // auto-generated file (unicode tables, transpiled output, generated stubs)
        // is not authored voice — its functions are neither reinvention candidates
        // nor placement candidates. Skips the F2 over-fire clean-commit measurement
        // caught on generated data modules (e.g. rich/_unicode_data).
        if adapter.is_data_dominant(&source, detect.data_threshold)
            || adapter.is_auto_generated(&source, &detect.generated_markers)
        {
            continue;
        }
        let mut added: HashSet<usize> = HashSet::new();
        for h in &batch.hunks {
            for l in h.new_start..(h.new_start + h.new_lines) {
                added.insert(l as usize);
            }
        }
        for f in functions_in_file(adapter.as_ref(), &batch.file_path, &source) {
            if added.contains(&f.line) {
                candidates.push((bi, lang, f));
            }
        }
    }
    if candidates.is_empty() {
        return Vec::new();
    }
    t_cand.done();

    // Load only the indices we actually need.
    let t_idx = crate::timing::phase("check: semantic index decode");
    let mut loaded: HashMap<&'static str, LoadedIndex> = HashMap::new();
    for (_, lang, _) in &candidates {
        if loaded.contains_key(lang) {
            continue;
        }
        match artifact.load(lang) {
            Ok(Some(li)) => {
                loaded.insert(lang, li);
            }
            Ok(None) => {}
            Err(e) => stderr.push_str(&format!("[argot] semantic index for {lang}: {e}\n")),
        }
    }
    candidates.retain(|(_, lang, _)| loaded.contains_key(lang));
    if candidates.is_empty() {
        return Vec::new();
    }
    t_idx.done();

    // Acquire the embedder once; unavailable model → degrade (no semantic hits).
    let t_model = crate::timing::phase("check: semantic embedder load");
    let embedder = match Embedder::ready() {
        Ok(Some(e)) => e,
        Ok(None) => {
            stderr.push_str(
                "[argot] semantic model unavailable — redundant/misplaced checks skipped this run\n",
            );
            return Vec::new();
        }
        Err(e) => {
            stderr.push_str(&format!("[argot] semantic model load failed: {e}\n"));
            return Vec::new();
        }
    };

    t_model.done();

    // Embed all candidate functions in one batch, serving any the machine-wide
    // cache already holds (e.g. functions a fit of this repo indexed at HEAD).
    let t_embed = crate::timing::phase(format!("check: semantic embed ({} fns)", candidates.len()));
    let embed_cache = crate::scoring::semantic::embed_cache::EmbedCache::open_current();
    let texts: Vec<&str> = candidates.iter().map(|(_, _, f)| f.text.as_str()).collect();
    let vecs = match crate::scoring::semantic::embed_cache::embed_with_cache(
        &embedder,
        &texts,
        embed_cache.as_ref(),
    ) {
        Ok(v) => v,
        Err(e) => {
            stderr.push_str(&format!("[argot] semantic embedding failed: {e}\n"));
            return Vec::new();
        }
    };
    t_embed.done();
    let _t_score = crate::timing::phase("check: semantic score candidates");

    // Dev-only feature capture (`ARGOT_SEM_DUMP=<path>`): append one JSON line
    // per candidate — its structural features, nearest neighbours and the fire
    // outcome — so bench sweeps can re-evaluate rule variants offline against a
    // saved index copy without re-running fit/check. Inert without the env var.
    let dump_path = std::env::var_os("ARGOT_SEM_DUMP");
    let mut dump_lines: Vec<String> = Vec::new();

    // Scorer construction is per-language, never per-candidate:
    // `RedundantScorer::new` builds corpus-wide IDF/DF tables over the whole
    // index — rebuilt for every candidate it dominated the check phase
    // (~35 ms × every diff-defined function on a 25k-entry index).
    let scorers: HashMap<&'static str, (RedundantScorer, PlacementScorer)> = loaded
        .iter()
        .map(|(lang, li)| {
            (
                *lang,
                (
                    RedundantScorer::new(&li.index, &li.reinvention),
                    PlacementScorer::new(&li.index, &li.placement),
                ),
            )
        })
        .collect();

    // Evaluate all candidates in parallel: the scorers are read-only, each
    // candidate is independent, and results come back in candidate order with
    // F1-before-F2 preserved per candidate — element-for-element identical to
    // the sequential loop.
    let evals = crate::par::par_map_indexed(candidates.len(), |i| {
        let (_, lang, f) = &candidates[i];
        let (redundant, placement) = &scorers[lang];
        let found = redundant.evaluate(f, &vecs[i]);
        // F2 placement is consulted only when F1 didn't claim the function.
        let mis = if found.is_none() {
            placement.evaluate(f, &vecs[i])
        } else {
            None
        };
        (found, mis)
    });

    let mut hits = Vec::new();
    for (((bi, lang, f), vec), (found, mis)) in candidates.iter().zip(&vecs).zip(evals) {
        let li = &loaded[lang];
        let batch = &patches[*bi];
        let mut fired: Option<&'static str> = None;
        // F1 first: a duplicate isn't "misplaced", it's "redundant" — the
        // stronger signal wins, one finding per function.
        if let Some(found) = found {
            fired = Some("redundant");
            let similarity = found.similarity;
            hits.push(build_semantic_hit(
                batch,
                f,
                "redundant",
                similarity as f64,
                crate::scoring::semantic::redundant::MIN_SIMILARITY_TO_FIRE as f64,
                SemanticHitEvidence::Redundant {
                    nearest_symbol: found.nearest_symbol,
                    nearest_path: found.nearest_path,
                    nearest_line: found.nearest_line,
                    similarity,
                },
                filter_adapters,
                mute_rules,
            ));
        }
        // F2 placement (only when F1 didn't already claim the function).
        if fired.is_none() {
            if let Some(m) = mis {
                fired = Some("misplaced");
                let score = (m.expected_fraction - m.in_area_fraction).max(0.0) as f64;
                hits.push(build_semantic_hit(
                    batch,
                    f,
                    "misplaced",
                    score,
                    m.expected_fraction as f64,
                    SemanticHitEvidence::Misplaced {
                        neighbor_area: m.neighbor_area,
                        actual_area: m.actual_area,
                        peers: m.peers,
                    },
                    filter_adapters,
                    mute_rules,
                ));
            }
        }
        if dump_path.is_some() {
            dump_lines.push(dump_semantic_candidate(lang, f, vec, li, fired));
        }
    }
    if let (Some(p), false) = (dump_path, dump_lines.is_empty()) {
        use std::io::Write as _;
        if let Ok(mut fh) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&p)
        {
            let _ = writeln!(fh, "{}", dump_lines.join("\n"));
        }
    }
    hits
}

/// One JSON line for the `ARGOT_SEM_DUMP` capture: the candidate's identity and
/// structural features, its f16 embedding, its top nearest index neighbours
/// (unfiltered — offline analysis applies its own same-file / cross-file rules
/// by joining `entry_index` with a saved copy of the index), and the production
/// fire outcome (to validate offline re-implementations against the binary).
#[cfg(feature = "semantic")]
fn dump_semantic_candidate(
    lang: &str,
    f: &crate::scoring::semantic::index::FunctionRef,
    vec: &[f32],
    li: &crate::scoring::semantic::index::LoadedIndex,
    fired: Option<&str>,
) -> String {
    use base64::Engine as _;
    /// Neighbours captured per candidate — enough headroom over the check-time
    /// k=10 for offline variants to re-filter (same-file, deeper k) losslessly.
    const DUMP_NEIGHBORS: usize = 40;
    let neighbors: Vec<serde_json::Value> = li
        .index
        .nearest(vec, DUMP_NEIGHBORS, |_| true)
        .iter()
        .map(|n| serde_json::json!([n.entry_index, n.cosine]))
        .collect();
    let vec_f16: Vec<u8> = vec
        .iter()
        .flat_map(|x| half::f16::from_f32(*x).to_le_bytes())
        .collect();
    serde_json::json!({
        "lang": lang,
        "path": f.path,
        "symbol": f.symbol,
        "line": f.line,
        "body_lines": f.text.lines().count(),
        "callees": f.callees,
        "subtokens": f.subtokens,
        "vec_b64": base64::engine::general_purpose::STANDARD.encode(vec_f16),
        "neighbors": neighbors,
        "fired": fired,
    })
    .to_string()
}

/// Build one semantic `Finding`, applying the mute + inline suppression
/// surfaces exactly as base hits do. `reason` is `"redundant"` / `"misplaced"`.
#[cfg(feature = "semantic")]
#[allow(clippy::too_many_arguments)]
fn build_semantic_hit(
    batch: &PatchBatch,
    f: &crate::scoring::semantic::index::FunctionRef,
    reason: &str,
    score: f64,
    threshold: f64,
    sem: SemanticHitEvidence,
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    mute_rules: &[SuppressionRule],
) -> Finding {
    let hunk_content = f.text.clone();
    let hash = hit_hash(&batch.file_path, reason, &hunk_content);
    let source = String::from_utf8_lossy(&batch.content);
    let suppressions = FileSuppressions::parse(
        &batch.file_path,
        &source,
        ext_to_lang(&extension(&batch.file_path))
            .and_then(|l| filter_adapters.get(l))
            .map(|a| a.line_comment_prefix()),
        mute_rules,
        false,
    );
    let suppressed_by = suppressions.classify(reason, &hash, f.line, f.end_line);
    Finding {
        score,
        file_path: batch.file_path.clone(),
        line: f.line,
        line_end: f.end_line,
        source: batch.source.clone(),
        reason: reason.to_string(),
        flagged: true,
        threshold,
        hunk_content,
        evidence: Some(Box::new(sem)),
        hash,
        suppressed_by,
    }
}

#[cfg(feature = "semantic")]
impl RenderEvidence for SemanticHitEvidence {
    fn human(&self, use_color: bool, _hunk_start_line: usize) -> Vec<String> {
        format_semantic_evidence(self, use_color)
    }

    fn machine(&self, _hunk_start_line: usize) -> Vec<String> {
        format_semantic_evidence(self, false)
            .into_iter()
            .map(|l| l.trim().to_string())
            .collect()
    }

    fn similarity(&self) -> Option<f32> {
        match self {
            SemanticHitEvidence::Redundant { similarity, .. } => Some(*similarity),
            SemanticHitEvidence::Misplaced { .. } => None,
        }
    }
}

/// Render the nearest-existing-code evidence for a semantic finding as `↳` lines
/// (F4 — retrieval + template, no LLM).
#[cfg(feature = "semantic")]
fn format_semantic_evidence(sem: &SemanticHitEvidence, use_color: bool) -> Vec<String> {
    match sem {
        SemanticHitEvidence::Redundant {
            nearest_symbol,
            nearest_path,
            nearest_line,
            similarity,
        } => {
            let body = format!(
                "    ↳ duplicates {nearest_symbol} ({nearest_path}:{nearest_line}) — similarity {similarity:.2}"
            );
            vec![paint(&body, C_DIM, use_color)]
        }
        SemanticHitEvidence::Misplaced {
            neighbor_area,
            actual_area,
            peers,
        } => {
            let filed = if actual_area.is_empty() {
                "the repo root".to_string()
            } else {
                actual_area.clone()
            };
            let head = format!("    ↳ looks like {neighbor_area} code filed under {filed}");
            let mut lines = vec![paint(&head, C_DIM, use_color)];
            if let Some((sym, path, line)) = peers.first() {
                let peer = format!("      nearest peer: {sym} ({path}:{line})");
                lines.push(paint(&peer, C_DIM, use_color));
            }
            lines
        }
    }
}

/// Build the eslint-style `^^^^^` underline for one source line
/// (`_render_caret_line`, `use_color=false`). Column ranges are byte offsets;
/// overlapping spans merge; returns `None` when no caret ends up printable.
fn render_caret_line(
    raw_line: &str,
    spans: &[SourceSpan],
    visible_prefix_width: usize,
) -> Option<String> {
    let line_len = raw_line.len(); // byte length, matching the byte-offset spans
    let mut covered = vec![false; line_len];
    for sp in spans {
        let end = sp.col_end.min(line_len);
        for c in covered.iter_mut().take(end).skip(sp.col_start) {
            *c = true;
        }
    }
    if !covered.iter().any(|&c| c) {
        return None;
    }
    let underline: String = covered.iter().map(|&c| if c { '^' } else { ' ' }).collect();
    let underline = underline.trim_end();
    if underline.is_empty() {
        return None;
    }
    Some(format!("{}{}", " ".repeat(visible_prefix_width), underline))
}

/// Format the hunk body as a numbered code block (`_render_hunk_body`,
/// `use_color=false`). `max_lines = None` in verbose mode. `must_show_hunk_lines`
/// grows the truncation budget to keep flagged lines in-frame;
/// `caret_spans_by_line` draws `^^^^` underlines below flagged source lines.
/// Returns `(lines, overflow)`.
fn render_hunk_body(
    content: &str,
    start_line: usize,
    max_lines: Option<usize>,
    must_show_hunk_lines: &HashSet<usize>,
    caret_spans_by_line: &HashMap<usize, Vec<SourceSpan>>,
    use_color: bool,
    caret_color: &str,
) -> (Vec<String>, usize) {
    if let Some(n) = max_lines {
        if n == 0 {
            return (Vec::new(), splitlines(content).len());
        }
    }
    let raw_lines = splitlines(content);
    if raw_lines.is_empty() {
        return (Vec::new(), 0);
    }
    let shown = match max_lines {
        None => raw_lines.len(),
        Some(n) => {
            let mut shown = n.min(raw_lines.len());
            // Smart-peek: grow the budget so any flagged hunk-relative line is
            // in-frame, bounded by the actual hunk length.
            let max_in_range = must_show_hunk_lines
                .iter()
                .copied()
                .filter(|&ln| 1 <= ln && ln <= raw_lines.len())
                .max();
            if let Some(m) = max_in_range {
                shown = raw_lines.len().min(shown.max(m));
            }
            shown
        }
    };
    let overflow = raw_lines.len() - shown;
    let width = (start_line + shown - 1).to_string().len();
    // Visible-prefix width for caret alignment: "  " + ln digits + " " + "|" + " ".
    let caret_pad = 2 + width + 1 + 1 + 1;
    let mut out: Vec<String> = Vec::new();
    for (i, line) in raw_lines.iter().take(shown).enumerate() {
        let ln = start_line + i;
        out.push(format!("  {:>width$} | {}", ln, line, width = width));
        // The i-th rendered line is hunk-line (i + 1) regardless of start_line.
        if let Some(spans) = caret_spans_by_line.get(&(i + 1)) {
            if let Some(caret) = render_caret_line(line, spans, caret_pad) {
                out.push(paint(&caret, caret_color, use_color));
            }
        }
    }
    if overflow > 0 {
        let plural = if overflow != 1 { "s" } else { "" };
        out.push(paint(
            &format!(
                "  {}   (+{} more line{})",
                " ".repeat(width),
                overflow,
                plural
            ),
            C_DIM,
            use_color,
        ));
    }
    (out, overflow)
}

/// Render grouped results (`_render_results`). Colored per-severity when
/// `use_color`; otherwise byte-identical to the parity fixtures. Returns whether
/// any hunk body was truncated.
fn render_results(
    hits: &[&Finding],
    hunk_lines: Option<usize>,
    use_color: bool,
    out: &mut String,
) -> bool {
    // Banner tier counts use the per-hit calibrated threshold.
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for h in hits {
        *counts
            .entry(confidence(&h.reason, h.score, h.threshold))
            .or_insert(0) += 1;
    }
    let total = hits.len();
    let mut tier_parts: Vec<String> = Vec::new();
    for tier in ["foreign", "suspicious", "unusual"] {
        let c = *counts.get(tier).unwrap_or(&0);
        if c > 0 {
            tier_parts.push(format!(
                "{c} {}",
                paint(tier, confidence_color(tier), use_color)
            ));
        }
    }
    let mut banner = format!(
        "argot check · {} hunk{} above threshold",
        total,
        if total != 1 { "s" } else { "" }
    );
    if !tier_parts.is_empty() {
        banner.push_str(&format!(" ({})", tier_parts.join(" · ")));
    }
    out.push_str(&banner);
    out.push('\n');
    out.push_str("note: argot is a probabilistic style linter — verify before action.\n");
    out.push('\n');

    // Group by file; file_max starts at 0.0 (defaultdict(float)) so all-negative
    // scores tie at 0.0 and files fall back to first-appearance (walk) order.
    let mut order: Vec<String> = Vec::new();
    let mut file_max: HashMap<String, f64> = HashMap::new();
    let mut file_hits: HashMap<String, Vec<&Finding>> = HashMap::new();
    for h in hits {
        if !file_hits.contains_key(&h.file_path) {
            order.push(h.file_path.clone());
        }
        let m = file_max.entry(h.file_path.clone()).or_insert(0.0);
        if h.score > *m {
            *m = h.score;
        }
        file_hits.entry(h.file_path.clone()).or_default().push(h);
    }
    let mut sorted_files = order;
    // Stable descending sort by file_max (ties keep insertion order).
    sorted_files.sort_by(|a, b| {
        file_max[b]
            .partial_cmp(&file_max[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut any_truncated = false;
    let n_files = sorted_files.len();
    for (i, fp) in sorted_files.iter().enumerate() {
        out.push_str(&paint(fp, C_BOLD, use_color));
        out.push('\n');

        let mut fhits: Vec<&Finding> = file_hits[fp].clone();
        fhits.sort_by_key(|h| h.line); // stable by line asc

        for h in &fhits {
            let sev = confidence(&h.reason, h.score, h.threshold);
            let color = confidence_color(sev);
            let line_str = if h.line == h.line_end {
                format!("L{}", h.line)
            } else {
                format!("L{}-L{}", h.line, h.line_end)
            };
            // The meta line names the rule (`foreign-import`, `redundant`, …);
            // internal reasons without a rule (`none` under --threshold) print raw.
            let meta = format!("· {} · {}", h.source, rules::code_for_reason(&h.reason));
            let glyph = match sev {
                "foreign" => "!",
                "suspicious" => "?",
                _ => ".",
            };
            // ANSI codes are zero-width, so the `{:<13}`/`{:>6.2}` columns still
            // align; only the glyph, severity word, and hash carry escapes.
            out.push_str(&format!(
                "  {}  {:<13} {:>6.2}  {}  {} {}\n",
                paint(glyph, color, use_color),
                line_str,
                h.score,
                paint(sev, color, use_color),
                meta,
                paint(&format!("[{}]", h.hash), C_DIM, use_color),
            ));

            // Rule-owned evidence sits between the headline and the hunk body.
            // `hunk_start_line = h.line` lets import evidence render `(L7)`
            // file-line annotations.
            if let Some(ev) = &h.evidence {
                for line in ev.human(use_color, h.line) {
                    out.push_str(&line);
                    out.push('\n');
                }
            }

            // Smart-peek keeps flagged lines in-frame; caret spans drive the
            // eslint-style `^^^^` underlines under the offending bytes.
            let must_show = h
                .evidence
                .as_ref()
                .map(|e| e.lines_of_interest())
                .unwrap_or_default();
            let caret_spans = h
                .evidence
                .as_ref()
                .map(|e| e.caret_spans())
                .unwrap_or_default();
            let (body, overflow) = render_hunk_body(
                &h.hunk_content,
                h.line,
                hunk_lines,
                &must_show,
                &caret_spans,
                use_color,
                color,
            );
            for line in body {
                out.push_str(&line);
                out.push('\n');
            }
            if overflow > 0 {
                any_truncated = true;
            }
        }

        if i < n_files - 1 {
            out.push('\n');
        }
    }

    any_truncated
}

/// Insert inline `argot: ignore-next-line` comments above the given 1-indexed
/// lines of `source`, bottom-up so earlier insertions never shift later
/// targets. Each comment copies the target line's indentation. Pure — the
/// caller does the I/O.
fn insert_ignore_comments(source: &str, comments: &[(usize, String)]) -> String {
    let mut lines: Vec<String> = source.split('\n').map(str::to_string).collect();
    let mut sorted: Vec<&(usize, String)> = comments.iter().collect();
    sorted.sort_by_key(|(line, _)| std::cmp::Reverse(*line));
    for (line, text) in sorted {
        let idx = line.saturating_sub(1).min(lines.len());
        let indent: String = lines
            .get(idx)
            .map(|l| l.chars().take_while(|c| c.is_whitespace()).collect())
            .unwrap_or_default();
        lines.insert(idx, format!("{indent}{text}"));
    }
    lines.join("\n")
}

/// `--add-ignores`: write one inline suppression above every visible finding
/// (deduped per line; a line carrying several rules gets one unscoped
/// comment). Adoption tooling — a wall of existing findings becomes a set of
/// reviewable, greppable comments instead of a red first run.
fn add_ignore_comments(
    args: &CheckArgs,
    visible: &[&Finding],
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    stderr: String,
) -> CheckOutcome {
    // Only the working-tree modes: editing files based on a historical ref's
    // line numbers would write comments into the wrong places.
    if !args.reference.is_empty() || args.commit.as_deref().is_some_and(|c| !c.is_empty()) {
        return CheckOutcome::err(
            "error: --add-ignores edits the working tree — run it without a ref/--commit\n"
                .to_string(),
            2,
        );
    }
    if visible.is_empty() {
        return CheckOutcome {
            stdout: "No findings — nothing to ignore.\n".to_string(),
            stderr,
            exit_code: 0,
        };
    }

    // file → line → rules found there.
    let mut by_file: BTreeMap<&str, BTreeMap<usize, Vec<&str>>> = BTreeMap::new();
    for h in visible {
        by_file
            .entry(h.file_path.as_str())
            .or_default()
            .entry(h.line)
            .or_default()
            .push(rules::code_for_reason(&h.reason));
    }

    let mut files_written = 0usize;
    let mut comments_written = 0usize;
    let mut stderr = stderr;
    for (file, lines) in &by_file {
        let Some(prefix) = ext_to_lang(&extension(file))
            .and_then(|l| filter_adapters.get(l))
            .map(|a| a.line_comment_prefix())
        else {
            stderr.push_str(&format!("[argot] {file}: unknown language — skipped\n"));
            continue;
        };
        let path = Path::new(&args.repo_path).join(file);
        let Ok(source) = fs::read_to_string(&path) else {
            stderr.push_str(&format!("[argot] {file}: unreadable — skipped\n"));
            continue;
        };
        let comments: Vec<(usize, String)> = lines
            .iter()
            .map(|(line, rule_names)| {
                let mut names: Vec<&str> = rule_names.clone();
                names.sort_unstable();
                names.dedup();
                let scope = if names.len() == 1 {
                    format!(" rule={}", names[0])
                } else {
                    String::new()
                };
                (
                    *line,
                    format!(
                        "{prefix} argot: ignore-next-line{scope} — baselined by --add-ignores; review"
                    ),
                )
            })
            .collect();
        let updated = insert_ignore_comments(&source, &comments);
        if let Err(e) = fs::write(&path, updated) {
            stderr.push_str(&format!("[argot] {file}: write failed ({e}) — skipped\n"));
            continue;
        }
        files_written += 1;
        comments_written += comments.len();
    }

    CheckOutcome {
        stdout: format!(
            "Added {comments_written} ignore comment(s) across {files_written} file(s) — \
             review them, then commit (each carries a greppable reason).\n"
        ),
        stderr,
        exit_code: 0,
    }
}

/// The check exit code for the visible findings: 1 when any finding's rule is
/// configured `error` (or when `--error-on-warnings` promotes a warn-only
/// run), 0 otherwise. Unregistered reasons gate as `error` — a finding never
/// silently loses its gate.
fn gate_exit_code(visible: &[&Finding], settings: &RuleSettings, error_on_warnings: bool) -> i32 {
    let fails = visible
        .iter()
        .any(|h| settings.severity_of_reason(&h.reason) == RuleSeverity::Error)
        || (error_on_warnings && !visible.is_empty());
    if fails {
        1
    } else {
        0
    }
}

/// Flatten visible hits into serializable [`HitRecord`]s for the machine
/// formats. Confidence is measured against the per-hit calibrated threshold,
/// matching the human rendering; severity is the rule's configured level;
/// evidence lines are the same per-reason lines the human path prints, with
/// layout indentation stripped.
fn hit_records(
    hits: &[&Finding],
    settings: &RuleSettings,
    registry: &rules::Registry,
) -> Vec<HitRecord> {
    hits.iter()
        .map(|h| HitRecord {
            path: h.file_path.clone(),
            line_start: h.line,
            line_end: h.line_end,
            score: h.score,
            threshold: h.threshold,
            confidence: confidence(&h.reason, h.score, h.threshold).to_string(),
            severity: settings.severity_of_reason(&h.reason).as_str().to_string(),
            rule: rules::code_for_reason(&h.reason).to_string(),
            rule_label: registry.label_for_reason(&h.reason).to_string(),
            source: h.source.clone(),
            hash: h.hash.clone(),
            evidence: h
                .evidence
                .as_ref()
                .map(|e| e.machine(h.line))
                .unwrap_or_default(),
            symbol: h.evidence.as_ref().and_then(|e| e.symbol()),
            // Verbatim, untruncated flagged specifiers for import findings —
            // machine consumers (e.g. `argot audit`) classify these without
            // re-parsing the rendered evidence, which caps the list at TOP_K.
            foreign_specifiers: h
                .evidence
                .as_ref()
                .map(|e| e.foreign_specifiers())
                .unwrap_or_default(),
            similarity: h.evidence.as_ref().and_then(|e| e.similarity()),
        })
        .collect()
}

fn report_meta(
    args: &CheckArgs,
    scanned: String,
    hunks_scanned: usize,
    files_scanned: Vec<FileScan>,
    model: &str,
) -> ReportMeta {
    ReportMeta {
        // The workspace shares one version across crates, so this matches the
        // CLI binary's version.
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        repo: args.repo_path.clone(),
        scanned,
        hunks_scanned,
        files_scanned,
        model: model.to_string(),
    }
}

/// Render the complete machine-format document (json/sarif) for stdout.
fn render_machine(format: OutputFormat, meta: &ReportMeta, records: &[HitRecord]) -> String {
    match format {
        OutputFormat::Sarif => render_sarif(meta, records),
        OutputFormat::Github => render_github(records),
        _ => render_json(meta, records),
    }
}

/// Freshness walks stop visiting commits here — far past every threshold.
/// The stale-after threshold itself is `[fit] refresh-after` in argot.toml.
pub const FRESHNESS_SCAN_CAP: usize = 200;

/// How many commits HEAD is ahead of the fit SHA (`None` when either end
/// cannot be resolved — shallow clones, rewritten history, detached states
/// must never break check). Public: the CLI's auto-refresh reads the same
/// staleness the in-check warning does.
pub fn commits_since_fit(repo_path: &str, fit_sha: &str) -> Option<usize> {
    let repo = open_repo(repo_path).ok()?;
    let head = repo.head().ok()?.peel_to_commit().ok()?;
    let fit_oid = git2::Oid::from_str(fit_sha).ok()?;
    if head.id() == fit_oid {
        return Some(0);
    }
    repo.find_commit(fit_oid).ok()?;
    let (ahead, _) = repo.graph_ahead_behind(head.id(), fit_oid).ok()?;
    Some(ahead)
}

/// The repo's default branch, by shorthand name — `origin/HEAD`'s target when
/// the remote declares one, else a local `main`/`master`. `None` when neither
/// exists (unusual layouts keep today's HEAD-relative behaviour).
fn default_branch_shorthand(repo: &git2::Repository) -> Option<String> {
    if let Ok(r) = repo.find_reference("refs/remotes/origin/HEAD") {
        if let Some(target) = r.symbolic_target() {
            if let Some(name) = target.strip_prefix("refs/remotes/origin/") {
                return Some(name.to_string());
            }
        }
    }
    ["main", "master"]
        .iter()
        .find(|name| repo.find_reference(&format!("refs/heads/{name}")).is_ok())
        .map(|s| s.to_string())
}

/// The trunk whose line counts as accepted history: the branch named in
/// `[fit] refresh-from` when it exists (locally or on origin), else the
/// auto-detected default branch — a named trunk missing from this clone
/// (a fork, a typo) degrades to detection rather than silently anchoring
/// at HEAD.
fn trunk_shorthand(repo: &git2::Repository, config: &crate::config::ArgotConfig) -> Option<String> {
    if let crate::config::FitRefreshFrom::Branch(name) = &config.fit_refresh_from {
        let exists = repo.find_reference(&format!("refs/heads/{name}")).is_ok()
            || repo
                .find_reference(&format!("refs/remotes/origin/{name}"))
                .is_ok();
        if exists {
            return Some(name.clone());
        }
    }
    default_branch_shorthand(repo)
}

/// The newest **accepted** commit the current work builds on. On the trunk
/// (or when no trunk is discernible) that's HEAD; on any other branch it's
/// the merge-base with the trunk. Feature-branch commits are deliberately not
/// accepted history — a voice refreshed against this anchor never learns
/// unreviewed work-in-progress, so `check` keeps judging it instead of
/// treating it as the repo's own. `None` when history can't be resolved
/// (shallow clones, disjoint roots).
pub fn accepted_anchor(repo_path: &str, config: &crate::config::ArgotConfig) -> Option<String> {
    let repo = open_repo(repo_path).ok()?;
    let head_ref = repo.head().ok()?;
    let head = head_ref.peel_to_commit().ok()?;
    let Some(trunk) = trunk_shorthand(&repo, config) else {
        return Some(head.id().to_string());
    };
    if head_ref.is_branch() && head_ref.shorthand() == Some(trunk.as_str()) {
        return Some(head.id().to_string());
    }
    let tip = repo
        .find_reference(&format!("refs/heads/{trunk}"))
        .or_else(|_| repo.find_reference(&format!("refs/remotes/origin/{trunk}")))
        .ok()?
        .peel_to_commit()
        .ok()?
        .id();
    let base = repo.merge_base(head.id(), tip).ok()?;
    Some(base.to_string())
}

/// How many commits in `from..to` touch corpus source under the given
/// suppressions — the staleness measure freshness decisions run on: docs,
/// CI config, and changelog churn don't age a voice; accepted source changes
/// do. Bounded twice so it never weighs on check: the count stops at
/// `stop_at` (callers only need "did it cross the threshold"), and the walk
/// itself gives up after [`FRESHNESS_SCAN_CAP`] commits (a fit that far
/// behind is stale regardless of the exact count). `None` when either end is
/// unresolvable — callers must leave the fit alone rather than guess.
pub fn in_scope_commits_between(
    repo_path: &str,
    from_sha: &str,
    to_sha: &str,
    suppressions: &crate::suppress::PathSuppressions,
    stop_at: usize,
) -> Option<usize> {
    let repo = open_repo(repo_path).ok()?;
    let from = git2::Oid::from_str(from_sha).ok()?;
    let to = git2::Oid::from_str(to_sha).ok()?;
    if from == to || stop_at == 0 {
        return Some(0);
    }
    repo.find_commit(from).ok()?;
    let mut walk = repo.revwalk().ok()?;
    walk.push(to).ok()?;
    walk.hide(from).ok()?;
    let mut in_scope = 0usize;
    for oid in walk.flatten().take(FRESHNESS_SCAN_CAP) {
        let commit = repo.find_commit(oid).ok()?;
        let tree = commit.tree().ok()?;
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
        let diff = repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
            .ok()?;
        let touches = diff.deltas().any(|d| {
            d.new_file()
                .path()
                .or(d.old_file().path())
                .and_then(|p| p.to_str())
                .is_some_and(|rel| crate::train::is_corpus_source(rel, suppressions))
        });
        if touches {
            in_scope += 1;
            if in_scope >= stop_at {
                break;
            }
        }
    }
    Some(in_scope)
}

/// The anchor freshness is measured against — and the commit a background
/// refresh fits at. [`accepted_anchor`] under the default
/// `[fit] refresh-from = "default-branch"`; plain HEAD when the repo opted
/// into `"current-branch"`.
pub fn freshness_anchor(repo_path: &str, config: &crate::config::ArgotConfig) -> Option<String> {
    match &config.fit_refresh_from {
        crate::config::FitRefreshFrom::DefaultBranch | crate::config::FitRefreshFrom::Branch(_) => {
            accepted_anchor(repo_path, config)
        }
        crate::config::FitRefreshFrom::CurrentBranch => {
            let repo = open_repo(repo_path).ok()?;
            let head = repo.head().ok()?.peel_to_commit().ok()?;
            Some(head.id().to_string())
        }
    }
}

/// The laundering advisory's evidence: when HEAD is a named branch other than
/// the default and its unmerged commits touch in-scope source, returns
/// `(branch, count)` (count stops at `cap`). `None` whenever a fit here is
/// unremarkable — on the default branch, detached HEAD (replay worktrees),
/// a branch with nothing in-scope of its own, or a repo that opted into
/// `[fit] refresh-from = "current-branch"`.
pub fn unmerged_branch_source_commits(
    repo_path: &str,
    config: &crate::config::ArgotConfig,
    cap: usize,
) -> Option<(String, usize)> {
    if config.fit_refresh_from == crate::config::FitRefreshFrom::CurrentBranch {
        return None;
    }
    let repo = open_repo(repo_path).ok()?;
    let head_ref = repo.head().ok()?;
    if !head_ref.is_branch() {
        return None;
    }
    let branch = head_ref.shorthand()?.to_string();
    if branch == trunk_shorthand(&repo, config)? {
        return None;
    }
    let head_sha = head_ref.peel_to_commit().ok()?.id().to_string();
    let anchor = accepted_anchor(repo_path, config)?;
    if anchor == head_sha {
        return None;
    }
    let n = in_scope_commits_between(
        repo_path,
        &anchor,
        &head_sha,
        &config.path_suppressions(),
        cap,
    )?;
    (n > 0).then_some((branch, n))
}

/// The shared freshness measure: commits of **accepted, in-scope** source the
/// fit hasn't seen — [`freshness_anchor`] composed with
/// [`in_scope_commits_between`]. Both check's drift warning and the CLI's
/// background auto-refresh read this, so a feature branch full of its own
/// commits reads as fresh (nothing accepted moved), and a docs-only sprint on
/// main does too. Cost on the check path: a couple of ref lookups plus one
/// commit-graph count; the per-commit tree diffs only run when accepted
/// history actually moved, and stop at `stop_at`.
pub fn accepted_source_commits_behind(
    repo_path: &str,
    fit_sha: &str,
    config: &crate::config::ArgotConfig,
    stop_at: usize,
) -> Option<usize> {
    let anchor = freshness_anchor(repo_path, config)?;
    // Cheap gate: no commits at all between fit and anchor (the common,
    // fresh case) answers 0 without a single tree diff.
    let repo = open_repo(repo_path).ok()?;
    let fit = git2::Oid::from_str(fit_sha).ok()?;
    let anchor_oid = git2::Oid::from_str(&anchor).ok()?;
    if fit == anchor_oid {
        return Some(0);
    }
    repo.find_commit(fit).ok()?;
    let (ahead, _) = repo.graph_ahead_behind(anchor_oid, fit).ok()?;
    if ahead == 0 {
        return Some(0);
    }
    in_scope_commits_between(
        repo_path,
        fit_sha,
        &anchor,
        &config.path_suppressions(),
        stop_at,
    )
}

/// Collect patches for the requested mode (`main()` mode dispatch). On a
/// mode-specific early exit returns the finished outcome.
fn collect_patches(args: &CheckArgs) -> Result<(Vec<PatchBatch>, String), CheckOutcome> {
    let repo_path = args.repo_path.as_str();
    let commit_set = args
        .commit
        .as_deref()
        .map(|c| !c.is_empty())
        .unwrap_or(false);
    let ref_nonempty = !args.reference.is_empty();

    if commit_set {
        let commit = args.commit.as_deref().unwrap();
        let repo =
            open_repo(repo_path).map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
        let shas = resolve_shas(&repo, commit)
            .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
        if shas.is_empty() {
            return Err(CheckOutcome::err(
                format!("No commits found for '{commit}'\n"),
                2,
            ));
        }
        let patches = committed_patches(repo_path, &shas)
            .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
        let short: String = commit.chars().take(8).collect();
        return Ok((patches, format!("1 commit ({short})")));
    }

    if ref_nonempty {
        let reference = args.reference.as_str();
        let repo =
            open_repo(repo_path).map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
        if let Some((base_raw, head_raw)) = reference.split_once("..") {
            // Score the *net* diff (merge-base → head), not each commit in the
            // range: a PR's voice check must match what a reviewer sees in the
            // files, so a fix commit clears an earlier commit's flag. Handles
            // both `a..b` and `a...b` (leading '.' of a three-dot range trimmed);
            // an empty side defaults to HEAD.
            let base = if base_raw.is_empty() {
                "HEAD"
            } else {
                base_raw
            };
            let head_trimmed = head_raw.trim_start_matches('.');
            let head = if head_trimmed.is_empty() {
                "HEAD"
            } else {
                head_trimmed
            };
            let patches = net_range_patches(repo_path, base, head)
                .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
            if patches.is_empty() {
                // Note: exit 0 (not 2) for an empty net diff.
                return Err(CheckOutcome::err(
                    format!("No changes in range '{reference}'\n"),
                    0,
                ));
            }
            return Ok((patches, format!("net diff ({reference})")));
        }
        // Bare ref: <ref>..HEAD commits plus full workdir. Validate the ref
        // first — otherwise `resolve_shas` treats an unknown start as "since the
        // beginning of history" and silently scores the whole tree as if clean.
        if repo.revparse_single(reference).is_err() {
            return Err(CheckOutcome::err(
                format!("error: unknown revision '{reference}' — not a commit, branch, or tag.\n"),
                2,
            ));
        }
        let shas = resolve_shas(&repo, &format!("{reference}..HEAD"))
            .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 2))?;
        let workdir = chain_workdir_patches(repo_path)
            .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
        if !shas.is_empty() {
            let mut patches = committed_patches(repo_path, &shas)
                .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
            patches.extend(workdir);
            return Ok((
                patches,
                format!("workdir + {} commit(s) since {reference}", shas.len()),
            ));
        }
        return Ok((workdir, format!("workdir (no commits since {reference})")));
    }

    if args.staged {
        let patches =
            staged_patches(repo_path).map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
        return Ok((patches, "staged changes".to_string()));
    }
    if args.unstaged {
        let patches = modified_patches(repo_path)
            .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
        return Ok((patches, "unstaged changes".to_string()));
    }

    let patches = chain_workdir_patches(repo_path)
        .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
    Ok((patches, "workdir".to_string()))
}

/// Entry point (`check.py:main`). Never exits the process — returns the outcome.
pub fn run_check(args: CheckArgs) -> CheckOutcome {
    // Mutual-exclusion validation — fail fast with a clear message (exit 2).
    let ref_nonempty = !args.reference.is_empty();
    let commit_set = args
        .commit
        .as_deref()
        .map(|c| !c.is_empty())
        .unwrap_or(false);
    if args.staged && args.unstaged {
        return CheckOutcome::err(
            "error: --staged and --unstaged are mutually exclusive\n".to_string(),
            2,
        );
    }
    if commit_set && ref_nonempty {
        return CheckOutcome::err(
            "error: --commit and ref positional are mutually exclusive\n".to_string(),
            2,
        );
    }
    if commit_set && (args.staged || args.unstaged) {
        return CheckOutcome::err(
            "error: --commit is mutually exclusive with --staged/--unstaged\n".to_string(),
            2,
        );
    }
    if ref_nonempty && (args.staged || args.unstaged) {
        return CheckOutcome::err(
            "error: ref positional is mutually exclusive with --staged/--unstaged\n".to_string(),
            2,
        );
    }

    // argot.toml config: excludes + `[detect]` heuristics + `[rules]` +
    // `[[mute]]`. Loaded once here — the `[detect]` markers gate the check-time
    // auto-generated skip built into each scorer, so they must be in place
    // before load_scorers.
    // The run's rule vocabulary: built-ins plus (in a scripted-rules build)
    // whatever `.argot/rules/` carries — discovered before config validation
    // so custom [rules] keys and severities resolve like built-in ones.
    let registry = rules::Registry::builtin();
    let config = ArgotConfig::load_with(Path::new(&args.repo_path), registry);
    // Effective per-rule severities: defaults ⊕ [rules] ⊕ CLI --rule overrides.
    let settings = config.rule_settings_with(registry, &args.rule_overrides);

    let t_load = crate::timing::phase("check: load scorers");
    let Loaded {
        mut scorers,
        filter_adapters,
        language_extensions,
        fit_sha,
        model_hash,
        slices,
        new_file_thresholds,
        fit_corpus_files,
    } = match load_scorers(&args.argot_dir, &config.detect) {
        Ok(l) => l,
        Err((msg, code)) => return CheckOutcome::err(msg, code),
    };
    t_load.done();

    let t_patches = crate::timing::phase("check: collect patches");
    let (patches, scan_label) = match collect_patches(&args) {
        Ok(v) => v,
        Err(outcome) => {
            // Machine formats own stdout: the only non-error early exit (an
            // explicit range with no commits, exit 0) still emits a complete,
            // hit-free document. Hard errors (exit != 0) stay stderr-only.
            if args.format.is_machine() && outcome.exit_code == 0 {
                let meta = report_meta(
                    &args,
                    format!("0 commit(s) ({})", args.reference),
                    0,
                    Vec::new(),
                    &model_hash,
                );
                return CheckOutcome {
                    stdout: render_machine(args.format, &meta, &[]),
                    stderr: outcome.stderr,
                    exit_code: 0,
                };
            }
            return outcome;
        }
    };
    t_patches.done();

    let mut stderr = String::new();

    // Name the model that judged this diff — reproducibility + "is my model the
    // same as my colleague's?". On stderr (human) so stdout stays byte-parity;
    // machine formats carry it in the report meta instead.
    if !args.format.is_machine() {
        stderr.push_str(&format!("[argot] model: {model_hash}\n"));
    }

    // Freshness: a stale model turns ordinary drift into noise (a month of
    // drift on a busy workspace measured ~14× the hit volume of a fresh
    // fit). Warn when ACCEPTED history has moved substantially since the fit
    // — commits touching in-scope source on the default-branch line. A
    // feature branch's own commits don't count (they're the code under
    // judgment, not the voice), and docs-only churn doesn't either.
    if let Some(fit_sha) = &fit_sha {
        let stale_after = config.fit_refresh_after;
        if let Some(behind) =
            accepted_source_commits_behind(&args.repo_path, fit_sha, &config, stale_after)
        {
            if behind >= stale_after {
                // No imperative here: the CLI's auto-refresh acts on this
                // drift itself (and says so right after this line).
                stderr.push_str(&format!(
                    "[argot] model fitted {behind}+ source commits ago — voice may have drifted\n"
                ));
            }
        }
    }

    // Fit-time health (persisted by the last fit — foreground OR background,
    // whose stdout is detached): the "is it time to recalibrate?" answer,
    // surfaced by the command users actually run.
    if let Some(health) = crate::health::read(&args.argot_dir) {
        if !health.drift_candidates.is_empty() {
            let shown: Vec<&str> = health
                .drift_candidates
                .iter()
                .take(3)
                .map(String::as_str)
                .collect();
            let more = if health.drift_candidates.len() > 3 {
                ", …"
            } else {
                ""
            };
            stderr.push_str(&format!(
                "[argot] {} director{} look generated or data-heavy and are shaping the voice                  ({}{more}) — review `argot init --suggest`
",
                health.drift_candidates.len(),
                if health.drift_candidates.len() != 1 {
                    "ies"
                } else {
                    "y"
                },
                shown.join(", "),
            ));
        }
        if !health.config_fingerprint.is_empty()
            && health.config_fingerprint != crate::health::config_fingerprint(&config)
        {
            stderr.push_str(
                "[argot] argot.toml changed since the last fit — the voice doesn't reflect                  your configuration yet (auto-refresh will refit, or run `argot fit`)
",
            );
        }
    }

    // Suppression surfaces from argot.toml (config loaded above): the resolved
    // path set (recommended built-ins + `[exclude].paths`, the same set
    // calibration samples from) and the `[[mute]]` rules (expiry vs `args.today`).
    for w in &config.warnings {
        stderr.push_str(&format!("[argot] {w}\n"));
    }
    let path_suppressions = config.path_suppressions();
    let mutes = config.mutes(&args.today);
    for w in &mutes.warnings {
        stderr.push_str(&format!("[argot] {w}\n"));
    }

    // A supported language with no model in this fit is silently dropped by
    // batch_scope below — correct scoring, but the user must know their new
    // Go file has zero coverage until the next fit. (Computed pre-filter:
    // those batches don't survive it.)
    {
        let mut unfitted: Vec<&str> = patches_langs_without_model(&patches, &scorers);
        unfitted.sort_unstable();
        unfitted.dedup();
        if !unfitted.is_empty() {
            stderr.push_str(&format!(
                "[argot] this change touches {} file(s) — no model in the current fit;                  run `argot fit` to cover them
",
                unfitted.join("/"),
            ));
        }
    }

    // Scope + only/exclude filters. User-ignored files stay scored (marked) so
    // their suppressed hits are countable.
    let filtered: Vec<PatchBatch> = patches
        .into_iter()
        .filter_map(|mut b| {
            match batch_scope(&b.file_path, &language_extensions, &path_suppressions) {
                BatchScope::Drop => return None,
                BatchScope::ScoreSuppressed => b.ignored_by_pattern = true,
                BatchScope::Score => {}
            }
            passes_filters(&b.file_path, &args.only, &args.exclude).then_some(b)
        })
        .collect();

    // A supported language with no model in this fit gets silently dropped by
    // batch_scope — which is correct scoring, but the user must know their new
    // Go file has zero coverage until the next fit.
    {
        let mut unfitted: Vec<&str> = patches_langs_without_model(&filtered, &scorers);
        unfitted.sort_unstable();
        unfitted.dedup();
        if !unfitted.is_empty() {
            stderr.push_str(&format!(
                "[argot] this change touches {} file(s) — no model in the current fit;                  run `argot fit` to cover them
",
                unfitted.join("/"),
            ));
        }
    }

    // Changeset-wide local bindings: names any file in this change defines.
    // A change that calls what it also defines (a new feature naming its own
    // components) is new code, not foreign voice; only callees neither the
    // corpus nor the changeset knows keep contributing.
    let mut changeset_bindings: HashMap<&'static str, HashSet<String>> = HashMap::new();
    for b in &filtered {
        let ext = extension(&b.file_path);
        let Some(lang) = ext_to_lang(&ext) else {
            continue;
        };
        let Some(adapter) = filter_adapters.get(lang) else {
            continue;
        };
        let source = String::from_utf8_lossy(&b.content);
        changeset_bindings
            .entry(lang)
            .or_default()
            .extend(adapter.callable_definitions(&source));
    }
    for (lang, bindings) in changeset_bindings {
        if let Some(scorer) = scorers.get_mut(lang) {
            scorer.set_changeset_bindings(bindings);
        }
    }

    // `.h` routes to the same C/C++ model calibrate built it into (repo's
    // translation-unit majority) — computed once from the working tree.
    let header_cpp = crate::scoring::calibration::header_is_cpp(Path::new(&args.repo_path));

    // Register this run's detectors — the composition root. The rank pair is
    // the order table (see detector.rs): execution_rank runs additive passes
    // first and the base pass last (stderr interleave parity); merge_rank
    // puts the base pass's findings first (stdout parity). Deleting a rule
    // group deletes exactly its registration lines.
    let mut scan = ScanReport::default();
    let mut detectors: Vec<RegisteredDetector<'_>> = vec![RegisteredDetector {
        detector: Box::new(VoiceDetector {
            scorers: &mut scorers,
            slices: &slices,
            new_file_thresholds: &new_file_thresholds,
            fit_corpus_files: &fit_corpus_files,
        }),
        execution_rank: 3,
        merge_rank: 0,
    }];
    #[cfg(feature = "semantic")]
    detectors.push(RegisteredDetector {
        detector: Box::new(SemanticDetector),
        execution_rank: 0,
        merge_rank: 1,
    });
    #[cfg(feature = "arch")]
    detectors.push(RegisteredDetector {
        detector: Box::new(ArchDetector),
        execution_rank: 1,
        merge_rank: 2,
    });
    #[cfg(feature = "integrity")]
    detectors.push(RegisteredDetector {
        detector: Box::new(IntegrityDetector),
        execution_rank: 2,
        merge_rank: 3,
    });

    let hits = {
        let mut ctx = CheckContext {
            batches: &filtered,
            args: &args,
            filter_adapters: &filter_adapters,
            mute_rules: &mutes.active,
            detect: &config.detect,
            header_cpp,
            settings: &settings,
            stderr: &mut stderr,
            scan: &mut scan,
        };
        run_detectors(&mut detectors, &mut ctx)
    };
    drop(detectors);
    let ScanReport {
        hunk_count,
        files_scanned,
    } = scan;

    // A rule set to `off` emits nothing: its findings are dropped entirely
    // (an off rule inside an otherwise-enabled group reaches this filter;
    // internal reasons like `none` have no rule and always pass).
    let hits = {
        let mut hits = hits;
        hits.retain(|h| settings.severity_of_reason(&h.reason) != RuleSeverity::Off);
        hits
    };

    // Display gate: --threshold widens to every hit >= N; otherwise show flagged.
    let threshold_override = args.threshold;
    let above_all: Vec<&Finding> = if let Some(t) = threshold_override {
        hits.iter().filter(|h| h.score >= t).collect()
    } else {
        hits.iter().filter(|h| h.flagged).collect()
    };

    // Suppressed ≠ deleted: drop muted hits from output and exit-code
    // consideration, but say how many were muted (and by which surface).
    let (above, suppressed): (Vec<&Finding>, Vec<&Finding>) = above_all
        .into_iter()
        .partition(|h| h.suppressed_by.is_none());
    if !suppressed.is_empty() {
        let count = |s: SuppressedBy| {
            suppressed
                .iter()
                .filter(|h| h.suppressed_by == Some(s))
                .count()
        };
        stderr.push_str(&format!(
            "{} hits suppressed ({} by argot.toml excludes, {} inline, {} by argot.toml mutes)\n",
            suppressed.len(),
            count(SuppressedBy::Exclude),
            count(SuppressedBy::Inline),
            count(SuppressedBy::Mute),
        ));
    }

    // --min-confidence drops weaker tiers from both output and banner counts.
    let min_idx = confidence_index(&args.min_confidence);
    let visible: Vec<&Finding> = above
        .iter()
        .copied()
        .filter(|h| {
            let t = threshold_override.unwrap_or(h.threshold);
            confidence_index(confidence(&h.reason, h.score, t)) >= min_idx
        })
        .collect();

    // --add-ignores: edit the working tree instead of reporting.
    if args.add_ignores {
        return add_ignore_comments(&args, &visible, &filter_adapters, stderr);
    }

    // Cache the visible hits for `argot mute <hash>` — written on every check
    // run (best-effort; a read-only tree must not fail the check).
    let last_check: Vec<LastCheckHit> = visible
        .iter()
        .map(|h| LastCheckHit {
            path: h.file_path.clone(),
            reason: h.reason.clone(),
            hash: h.hash.clone(),
            line_start: h.line,
            line_end: h.line_end,
        })
        .collect();
    let _ = write_last_check(&args.argot_dir, &last_check);

    // Machine formats: the serialized document is the entire stdout; skip
    // warnings stay on stderr. Exit semantics match the human path (rule
    // severities decide, see gate_exit_code).
    if args.format.is_machine() {
        let records = hit_records(&visible, &settings, registry);
        let meta = report_meta(&args, scan_label, hunk_count, files_scanned, &model_hash);
        let mut stdout = render_machine(args.format, &meta, &records);
        // In the github format, the health notes ("model drifted", "config
        // changed since fit", "language not fitted") become run-level notices —
        // CI logs bury stderr, PR annotations don't.
        if args.format == OutputFormat::Github {
            for line in stderr.lines() {
                if let Some(note) = line.strip_prefix("[argot] ") {
                    stdout.push_str(&format!(
                        "::notice title=argot::{}
",
                        note.replace('%', "%25")
                    ));
                }
            }
        }
        return CheckOutcome {
            stdout,
            stderr,
            exit_code: gate_exit_code(&visible, &settings, args.error_on_warnings),
        };
    }

    if visible.is_empty() {
        let mut sorted_exts: Vec<&str> = SUPPORTED_EXTENSIONS.to_vec();
        sorted_exts.sort_unstable();
        let exts = sorted_exts.join(" ");
        let stdout = if hunk_count == 0 {
            format!(
                "No changes to supported files found ({scan_label} scanned).\nSupported extensions: {exts}\n"
            )
        } else if !above.is_empty() {
            format!(
                "All {} hit(s) below confidence '{}' — pass a lower --min-confidence to see them.\n",
                above.len(),
                args.min_confidence
            )
        } else if let Some(t) = threshold_override {
            format!("All {hunk_count} hunk(s) scored below threshold {t:.2} — looks clean.\n")
        } else {
            format!("All {hunk_count} hunk(s) scored below calibrated thresholds — looks clean.\n")
        };
        return CheckOutcome {
            stdout,
            stderr,
            exit_code: 0,
        };
    }

    let hunk_lines = if args.verbose {
        None
    } else {
        Some(args.hunk_lines)
    };
    let mut stdout = String::new();
    let any_truncated = render_results(&visible, hunk_lines, args.use_color, &mut stdout);

    if any_truncated && !args.verbose {
        stdout.push('\n');
        stdout.push_str("tip: pass --verbose (-v) to expand truncated hunks.\n");
    }

    CheckOutcome {
        stdout,
        stderr,
        exit_code: gate_exit_code(&visible, &settings, args.error_on_warnings),
    }
}

/// Outcome of `argot review-mutes` — mute-rot cleanup over the hash-scoped
/// `argot.toml` `[[mute]]` entries.
pub struct ReviewOutcome {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Re-run the check scoring over the files behind the currently-muted
/// hash-scoped mutes and report which no longer fire. With `prune`, stale hash
/// entries are removed from `argot.toml` (the `[[mute]]` array is rewritten;
/// expired and non-hash entries, and every other section, are kept).
///
/// A mute "still fires" when re-scoring the file's current content (as one
/// full-file hunk plus each sampleable range — stable, reproducible hunk
/// boundaries) yields a flagged hit with the entry's hash. Hits muted from
/// transient diff hunks whose boundaries no longer exist resolve as "no longer
/// fires" — which is exactly mute-rot.
pub fn run_review_mutes(repo_path: &str, today: &str, prune: bool) -> ReviewOutcome {
    let mut stdout = String::new();
    let mut stderr = String::new();

    let repo_root = Path::new(repo_path);
    let config = ArgotConfig::load(repo_root);
    for w in &config.warnings {
        stderr.push_str(&format!("[argot] {w}\n"));
    }
    let mutes = config.mutes(today);
    for w in &mutes.warnings {
        stderr.push_str(&format!("[argot] {w}\n"));
    }
    let hash_entries: Vec<&SuppressionRule> =
        mutes.active.iter().filter(|r| r.hash.is_some()).collect();
    if hash_entries.is_empty() {
        stdout.push_str("No hash-scoped suppressions to review.\n");
        return ReviewOutcome {
            stdout,
            stderr,
            exit_code: 0,
        };
    }

    stdout.push_str(&format!(
        "Reviewing {} hash-scoped suppression(s)…\n",
        hash_entries.len()
    ));
    // A hash-scoped mute names the exact file `argot mute` minted it from, and
    // its stored hash is a one-way digest of that specific diff hunk — there is
    // no way to recover the hunk from the hash, so re-scoring the live tree can
    // only *guess* at staleness (and would wrongly flag every mute of an
    // edited-but-still-present region, which `--prune` would then delete). The
    // one thing we can assert soundly is existence: a mute can never fire again
    // once its file is gone from both the working tree and HEAD. `--prune` acts
    // on that alone, so it never removes a mute still guarding live code.
    let mut dead_hashes: Vec<String> = Vec::new();
    for entry in &hash_entries {
        let hash = entry.hash.as_deref().expect("filtered on hash presence");
        let present = mute_path_present(repo_path, &entry.path);
        stdout.push_str(&format!(
            "  [{hash}]  {}  {}\n",
            entry.path,
            if present {
                "file present"
            } else {
                "file gone — dead"
            }
        ));
        if !present {
            dead_hashes.push(hash.to_string());
        }
    }

    if dead_hashes.is_empty() {
        stdout.push_str("Every muted file still exists — nothing to prune.\n");
    } else if prune {
        let mut kept: Vec<SuppressionRule> = Vec::new();
        for rule in mutes.active.iter().chain(mutes.expired.iter()) {
            let dead = rule
                .hash
                .as_deref()
                .is_some_and(|h| dead_hashes.iter().any(|s| s == h));
            if !dead {
                kept.push(rule.clone());
            }
        }
        match crate::config::write_mutes(repo_root, &kept) {
            Ok(()) => stdout.push_str(&format!(
                "Pruned {} dead mute(s) from argot.toml.\n",
                dead_hashes.len()
            )),
            Err(e) => {
                stderr.push_str(&format!("error: {e}\n"));
                return ReviewOutcome {
                    stdout,
                    stderr,
                    exit_code: 2,
                };
            }
        }
    } else {
        stdout.push_str(&format!(
            "{} dead mute(s) (file gone) — run `argot review-mutes --prune` to remove them.\n",
            dead_hashes.len()
        ));
    }

    ReviewOutcome {
        stdout,
        stderr,
        exit_code: 0,
    }
}

/// Does the repo still contain the file a hash-scoped mute names? `argot mute`
/// records the hit's exact path, so a plain path is checked against both the
/// working tree and `HEAD` — the mute is only "gone" when the file exists in
/// neither (a file still in HEAD can re-appear in a diff, so its mute is not
/// yet dead). A glob path (only ever hand-edited into a hash entry) is always
/// treated as present so `--prune` never reasons about a pattern.
fn mute_path_present(repo_path: &str, mute_path: &str) -> bool {
    if mute_path.contains(['*', '?', '[']) {
        return true;
    }
    if Path::new(repo_path).join(mute_path).is_file() {
        return true;
    }
    open_repo(repo_path)
        .ok()
        .and_then(|repo| {
            let tree = repo.head().ok()?.peel_to_commit().ok()?.tree().ok()?;
            Some(tree.get_path(Path::new(mute_path)).is_ok())
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suppress::parse_inline;

    #[test]
    #[cfg(feature = "arch")]
    fn arch_evidence_names_the_broken_direction() {
        use crate::scoring::arch_graph::Violation;
        let edge = ("core".to_string(), "cli".to_string());
        assert_eq!(
            arch_evidence(&edge, Violation::Reversal),
            "cli → core is this repo's direction — this import reverses it"
        );
        assert!(arch_evidence(&edge, Violation::TransitiveReversal).contains("closes a cycle"));
        assert!(arch_evidence(&edge, Violation::SinkOut).contains("never imports out of"));
    }

    #[test]
    fn insert_ignore_comments_bottom_up_with_indentation() {
        let src = "def a():\n    x = 1\n    y = 2\n\ndef b():\n    z = 3\n";
        let out = insert_ignore_comments(
            src,
            &[
                (2, "# argot: ignore-next-line — r1".to_string()),
                (
                    6,
                    "# argot: ignore-next-line rule=redundant — r2".to_string(),
                ),
            ],
        );
        let lines: Vec<&str> = out.lines().collect();
        // Indentation copied from the target line; both landed above their
        // original targets despite the insertions shifting line numbers.
        assert_eq!(lines[1], "    # argot: ignore-next-line — r1");
        assert_eq!(lines[2], "    x = 1");
        assert_eq!(
            lines[6],
            "    # argot: ignore-next-line rule=redundant — r2"
        );
        assert_eq!(lines[7], "    z = 3");
        // The inserted comments parse as real suppressions.
        let sup = parse_inline(&out, "#");
        assert_eq!(sup.rules.len(), 2);
        assert!(sup.warnings.is_empty());
    }

    #[test]
    fn integrity_reasons_have_labels_and_pinned_confidence() {
        assert_eq!(
            rules::label_for_reason("test_disabled"),
            "test disabled alongside code change"
        );
        assert_eq!(rules::code_for_reason("test_weakened"), "test-weakened");
        // Integrity findings are discrete evidenced events — mid tier.
        assert_eq!(confidence("test_deleted", 1.0, 0.5), "suspicious");
        assert_eq!(confidence("test_disabled", 1.0, 0.5), "suspicious");
        assert_eq!(confidence("test_weakened", 1.0, 0.5), "suspicious");
    }

    #[test]
    #[cfg(feature = "integrity")]
    fn integrity_pass_fires_on_a_staged_gaming_edit() {
        use std::process::Command;
        let root = &std::env::temp_dir().join(format!("argot_integrity_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(root);
        std::fs::create_dir_all(root).unwrap();
        let git = |args: &[&str]| {
            let ok = Command::new("git")
                .args(args)
                .current_dir(root)
                .env("GIT_AUTHOR_NAME", "t")
                .env("GIT_AUTHOR_EMAIL", "t@t")
                .env("GIT_COMMITTER_NAME", "t")
                .env("GIT_COMMITTER_EMAIL", "t@t")
                .output()
                .unwrap();
            assert!(ok.status.success(), "git {args:?}: {ok:?}");
        };
        git(&["init", "-q"]);
        std::fs::create_dir_all(root.join("tests")).unwrap();
        std::fs::write(
            root.join("parser.py"),
            "def parse(x):\n    return x.strip()\n",
        )
        .unwrap();
        std::fs::write(
            root.join("tests/test_parser.py"),
            "def test_parse():\n    assert parse(\" A \") == \"A\"\n    assert parse(\"\") == \"\"\n",
        )
        .unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-qm", "init"]);
        // Gaming edit: prod change + the failing assertion excised, staged.
        std::fs::write(
            root.join("parser.py"),
            "def parse(x):\n    return x.strip().lower()\n",
        )
        .unwrap();
        std::fs::write(
            root.join("tests/test_parser.py"),
            "def test_parse():\n    assert parse(\" A \") == \"A\"\n",
        )
        .unwrap();
        git(&["add", "-A"]);

        let args = CheckArgs {
            repo_path: root.to_string_lossy().to_string(),
            reference: String::new(),
            staged: true,
            unstaged: false,
            commit: None,
            only: Vec::new(),
            exclude: Vec::new(),
            threshold: None,
            argot_dir: root.join(".argot"),
            hunk_lines: 3,
            verbose: false,
            min_confidence: "unusual".to_string(),
            rule_overrides: Vec::new(),
            error_on_warnings: false,
            add_ignores: false,
            use_color: false,
            format: OutputFormat::Human,
            today: "2026-01-01".to_string(),
        };
        let adapters: HashMap<String, Box<dyn LanguageAdapter>> = HashMap::new();
        let mut stderr = String::new();
        // No artifact on disk → permissive default gates.
        let hits = integrity_hits(&args, &adapters, &[], &mut stderr);
        assert_eq!(hits.len(), 1, "stderr: {stderr}");
        let h = &hits[0];
        assert_eq!(h.reason, "test_weakened");
        assert_eq!(h.file_path, "tests/test_parser.py");
        assert!(h.flagged);
        let ev = h.evidence.as_ref().unwrap().machine(h.line).join("\n");
        assert!(ev.contains("test_parse"), "{ev}");
        assert!(ev.contains("parser.py"), "{ev}");
        // The affected test's name is surfaced as the finding's symbol.
        assert_eq!(
            h.evidence.as_ref().unwrap().symbol().as_deref(),
            Some("test_parse")
        );
        // A hit hash exists so `argot mute` can address it.
        assert_eq!(h.hash.len(), 12);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn semantic_reasons_have_labels_and_pinned_confidence() {
        assert_eq!(
            rules::label_for_reason("redundant"),
            "already implemented here"
        );
        assert_eq!(rules::label_for_reason("misplaced"), "unusual location");
        // Advisory findings are the mildest tier regardless of score.
        assert_eq!(confidence("redundant", 5.0, 0.1), "unusual");
        assert_eq!(confidence("misplaced", 5.0, 0.1), "unusual");
    }

    #[cfg(feature = "semantic")]
    #[test]
    fn semantic_evidence_renders_nearest_code() {
        let redundant = SemanticHitEvidence::Redundant {
            nearest_symbol: "slugify".into(),
            nearest_path: "src/utils/text.py".into(),
            nearest_line: 1,
            similarity: 0.86,
        };
        let lines = format_semantic_evidence(&redundant, false);
        assert!(lines[0].contains("duplicates slugify (src/utils/text.py:1)"));
        assert!(lines[0].contains("0.86"));

        let misplaced = SemanticHitEvidence::Misplaced {
            neighbor_area: "src/db".into(),
            actual_area: "src/ui".into(),
            peers: vec![("load_row".into(), "src/db/models.py".into(), 12)],
        };
        let lines = format_semantic_evidence(&misplaced, false);
        assert!(lines[0].contains("looks like src/db code filed under src/ui"));
        assert!(lines[1].contains("load_row (src/db/models.py:12)"));
    }

    #[test]
    fn foreign_import_tiers_as_foreign_regardless_of_margin() {
        // The import signal is categorical: score is a count of never-before-seen
        // modules against a threshold of 1.0, so a lone foreign import sits exactly
        // at the bar. It must still read as `foreign` — the strongest tier — not
        // fall through the BPE-margin logic into `unusual`.
        assert_eq!(confidence("import", 1.0, 1.0), "foreign");
        assert_eq!(confidence("import", 3.0, 1.0), "foreign");
    }

    #[test]
    fn distributional_signals_grade_by_margin() {
        // BPE / convention / call_receiver keep the additive-margin tiering, which
        // is calibrated for their nat-scale scores.
        let t = 8.0;
        assert_eq!(confidence("bpe", t, t), "unusual");
        assert_eq!(confidence("bpe", t + 0.5, t), "suspicious");
        assert_eq!(confidence("bpe", t + 1.5, t), "foreign");
        assert_eq!(confidence("call_receiver", t + 0.4, t), "unusual");
        assert_eq!(confidence("convention", t + 1.6, t), "foreign");
    }

    #[test]
    fn net_range_scores_the_pr_result_not_each_commit() {
        // base → (add file with a foreign import) → (rewrite it clean). The net
        // diff base..head is the clean file, so the reverted import must not
        // appear in the scored range — a fix commit clears a prior flag.
        let dir = std::env::temp_dir().join(format!("argot_netrange_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = git2::Repository::init(&dir).unwrap();
        std::fs::write(dir.join("keep.ts"), "export const x = 1\n").unwrap();
        let base = commit_all(&repo, "base");
        std::fs::write(
            dir.join("h.ts"),
            "import { Router } from 'express'\nexport const r = Router()\n",
        )
        .unwrap();
        commit_all(&repo, "add express handler");
        std::fs::write(
            dir.join("h.ts"),
            "import { Hono } from 'hono'\nexport const r = new Hono()\n",
        )
        .unwrap();
        let head = commit_all(&repo, "rewrite in hono style");

        let path = dir.to_str().unwrap();
        let patches = net_range_patches(path, &base.to_string(), &head.to_string()).unwrap();
        let h = patches
            .iter()
            .find(|p| p.file_path == "h.ts")
            .expect("h.ts in net diff");
        let content = String::from_utf8_lossy(&h.content);
        assert!(
            content.contains("Hono"),
            "net diff should carry the head content"
        );
        assert!(
            !content.contains("express"),
            "the reverted foreign import must not survive in the net range: {content}"
        );
    }

    fn commit_all(repo: &git2::Repository, msg: &str) -> git2::Oid {
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("t", "t@t").unwrap();
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents)
            .unwrap()
    }

    #[test]
    fn commits_since_fit_counts_head_distance() {
        let dir = std::env::temp_dir().join(format!("argot_freshness_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = git2::Repository::init(&dir).unwrap();
        std::fs::write(dir.join("a.py"), "x = 1\n").unwrap();
        let first = commit_all(&repo, "one");
        std::fs::write(dir.join("a.py"), "x = 2\n").unwrap();
        commit_all(&repo, "two");

        let path = dir.to_str().unwrap();
        assert_eq!(commits_since_fit(path, &first.to_string()), Some(1));
        let head = repo.head().unwrap().peel_to_commit().unwrap().id();
        assert_eq!(commits_since_fit(path, &head.to_string()), Some(0));
        // Unresolvable fit SHA must never break check.
        assert_eq!(commits_since_fit(path, "fixture"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn freshness_ignores_feature_branch_and_docs_churn() {
        let dir = std::env::temp_dir().join(format!("argot_anchor_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let repo = git2::Repository::init(&dir).unwrap();
        std::fs::write(dir.join("src/a.py"), "x = 1\n").unwrap();
        let c1 = commit_all(&repo, "c1: source on default");
        // Pin the default branch name regardless of the machine's git config.
        let c1_commit = repo.find_commit(c1).unwrap();
        repo.branch("main", &c1_commit, true).unwrap();
        repo.set_head("refs/heads/main").unwrap();

        // A feature branch with one source commit and one docs commit.
        repo.branch("feat", &c1_commit, true).unwrap();
        repo.set_head("refs/heads/feat").unwrap();
        std::fs::write(dir.join("src/b.py"), "y = 2\n").unwrap();
        commit_all(&repo, "c2: feature source");
        std::fs::write(dir.join("README.md"), "docs\n").unwrap();
        let c3 = commit_all(&repo, "c3: docs only");

        let path = dir.to_str().unwrap();
        let config = crate::config::ArgotConfig::default();

        // The anchor is the merge-base with main — the feature commits are
        // not accepted history.
        assert_eq!(accepted_anchor(path, &config), Some(c1.to_string()));
        // A voice fitted at the anchor is fresh no matter how busy the branch.
        assert_eq!(
            accepted_source_commits_behind(path, &c1.to_string(), &config, 10),
            Some(0)
        );
        // Of the branch's own commits, only the source one is in scope.
        assert_eq!(
            in_scope_commits_between(
                path,
                &c1.to_string(),
                &c3.to_string(),
                &config.path_suppressions(),
                10
            ),
            Some(1)
        );
        // The manual-fit advisory sees the same single unmerged source commit…
        assert_eq!(
            unmerged_branch_source_commits(path, &config, 10),
            Some(("feat".to_string(), 1))
        );
        // …and stays quiet when the repo opted into current-branch refreshes.
        let opt_dir = std::env::temp_dir().join(format!("argot_anchor_cfg_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&opt_dir);
        std::fs::create_dir_all(&opt_dir).unwrap();
        std::fs::write(
            opt_dir.join("argot.toml"),
            "[fit]\nrefresh-from = \"current-branch\"\n",
        )
        .unwrap();
        let opted_out = crate::config::ArgotConfig::load(&opt_dir);
        let _ = std::fs::remove_dir_all(&opt_dir);
        assert_eq!(
            opted_out.fit_refresh_from,
            crate::config::FitRefreshFrom::CurrentBranch
        );
        assert_eq!(unmerged_branch_source_commits(path, &opted_out, 10), None);
        // Under the opt-out the anchor is plain HEAD.
        assert_eq!(freshness_anchor(path, &opted_out), Some(c3.to_string()));

        // Back on the default branch: HEAD is the anchor, no advisory.
        repo.set_head("refs/heads/main").unwrap();
        assert_eq!(accepted_anchor(path, &config), Some(c1.to_string()));
        assert_eq!(unmerged_branch_source_commits(path, &config, 10), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A `[fit] refresh-from = "<branch>"` names the trunk explicitly for
    /// repos whose accepted line isn't main/master.
    #[test]
    fn named_trunk_overrides_default_branch_detection() {
        let dir = std::env::temp_dir().join(format!("argot_trunk_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let repo = git2::Repository::init(&dir).unwrap();
        std::fs::write(dir.join("src/a.py"), "x = 1\n").unwrap();
        let c1 = commit_all(&repo, "c1: trunk");
        let c1_commit = repo.find_commit(c1).unwrap();
        // Trunk is `develop`; no main/master exists anywhere.
        repo.branch("develop", &c1_commit, true).unwrap();
        repo.set_head("refs/heads/develop").unwrap();
        for stray in ["main", "master"] {
            if let Ok(mut b) = repo.find_branch(stray, git2::BranchType::Local) {
                b.delete().unwrap();
            }
        }
        repo.branch("feat", &c1_commit, true).unwrap();
        repo.set_head("refs/heads/feat").unwrap();
        std::fs::write(dir.join("src/b.py"), "y = 2\n").unwrap();
        let c2 = commit_all(&repo, "c2: feature source");

        let path = dir.to_str().unwrap();
        std::fs::write(
            dir.join("argot.toml"),
            "[fit]\nrefresh-from = \"develop\"\n",
        )
        .unwrap();
        let named = crate::config::ArgotConfig::load(&dir);
        assert_eq!(
            named.fit_refresh_from,
            crate::config::FitRefreshFrom::Branch("develop".to_string())
        );
        // Named trunk: the anchor is the merge-base with develop, and the
        // advisory sees the unmerged feature commit.
        assert_eq!(accepted_anchor(path, &named), Some(c1.to_string()));
        assert_eq!(
            unmerged_branch_source_commits(path, &named, 10),
            Some(("feat".to_string(), 1))
        );
        // Without the override there is no main/master to detect — the
        // anchor degrades to HEAD (today's behaviour for unusual layouts).
        let auto = crate::config::ArgotConfig::default();
        assert_eq!(accepted_anchor(path, &auto), Some(c2.to_string()));
        // A named trunk missing from the clone degrades to detection, not to
        // a silent HEAD anchor pretending the config was honored.
        std::fs::write(dir.join("argot.toml"), "[fit]\nrefresh-from = \"gone\"\n").unwrap();
        let missing = crate::config::ArgotConfig::load(&dir);
        assert_eq!(accepted_anchor(path, &missing), Some(c2.to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn in_scope_count_stops_at_threshold() {
        let dir = std::env::temp_dir().join(format!("argot_stopat_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let repo = git2::Repository::init(&dir).unwrap();
        std::fs::write(dir.join("src/a.py"), "x = 0\n").unwrap();
        let base = commit_all(&repo, "base");
        for i in 1..=5 {
            std::fs::write(dir.join("src/a.py"), format!("x = {i}\n")).unwrap();
            commit_all(&repo, &format!("c{i}"));
        }
        let head = repo.head().unwrap().peel_to_commit().unwrap().id();
        let path = dir.to_str().unwrap();
        let sup = crate::suppress::PathSuppressions::recommended();
        assert_eq!(
            in_scope_commits_between(path, &base.to_string(), &head.to_string(), &sup, 3),
            Some(3),
            "count is capped at stop_at"
        );
        assert_eq!(
            in_scope_commits_between(path, &base.to_string(), &head.to_string(), &sup, 10),
            Some(5)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
