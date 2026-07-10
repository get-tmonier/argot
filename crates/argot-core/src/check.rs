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
use crate::git_walk::{
    open_repo, resolve_shas, walk_commits, HunkSpan, WalkItem, SUPPORTED_EXTENSIONS,
};
use crate::output::{render_json, render_sarif, FileScan, HitRecord, OutputFormat, ReportMeta};
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
use crate::scoring::evidence::types::{Evidence, EvidenceCorpus, SourceSpan};
use crate::scoring::evidence::{evidence_caret_spans, evidence_lines_of_interest, format_evidence};
use crate::scoring::model::LanguageModel;
use crate::scoring::sequential::{ScoredHunk, SequentialConfig, SequentialImportBpeScorer};
use crate::suppress::{
    fnmatch, hit_hash, parse_inline, write_last_check, LastCheckHit, PathScope, PathSuppressions,
    SuppressionRule,
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
struct PatchBatch {
    file_path: String,
    content: Vec<u8>,
    hunks: Vec<HunkSpan>,
    source: String,
    /// The file matched a user `[exclude].paths` pattern: still scored (so the
    /// suppression is countable), but every hit is dropped from output and
    /// exit-code consideration.
    ignored_by_pattern: bool,
}

/// Which suppression surface muted a hit (`None` = reported normally).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SuppressedBy {
    /// An `argot.toml` `[exclude].paths` pattern.
    Exclude,
    /// An inline `# argot: ignore` comment.
    Inline,
    /// An `argot.toml` `[[mute]]` entry.
    Mute,
}

/// One above-threshold hunk plus everything needed to explain it (`_Hit`).
struct Hit {
    /// The winning candidate's score (adjusted for contributions), measured
    /// against the winning candidate's threshold — so severity tiers mean
    /// the same thing for every reason. A call-receiver hit that crossed on
    /// a +5 contribution reads as the strong signal it is, not as its raw
    /// BPE component.
    score: f64,
    file_path: String,
    line: usize,
    line_end: usize,
    source: String,
    reason: String,
    flagged: bool,
    threshold: f64,
    hunk_content: String,
    /// Per-reason evidence for the winning reason (`None` when the scorer had
    /// no `EvidenceCorpus`, or the hunk didn't fire a reason with a collector).
    evidence: Option<Evidence>,
    /// Content-based hit hash (path + winning reason + normalized hunk).
    hash: String,
    /// Set when a suppression surface muted this hit.
    suppressed_by: Option<SuppressedBy>,
    /// Nearest-code evidence for a semantic finding (reinvention / placement).
    /// Feature-gated so the base build has no extra field and stays byte-for-byte
    /// identical; base statistical Hits carry `None` when the feature is on.
    #[cfg(feature = "semantic")]
    semantic: Option<SemanticHitEvidence>,
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
///
/// Whether a finding fails the check is its rule's configured severity
/// (`error` / `warn` / `off`), not this tier.
fn confidence(reason: &str, score: f64, threshold: f64) -> &'static str {
    match reason {
        "import" => "foreign",
        "redundant" | "misplaced" | "layering" => "unusual",
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
/// set (recommended built-ins + `.argotignore` — the same set calibration
/// samples from; lock-step principle).
enum BatchScope {
    /// In scope: score and report normally.
    Score,
    /// In scope but matched by a user `.argotignore` pattern: score it so the
    /// suppression is countable, then drop its hits from output.
    ScoreSuppressed,
    /// Out of scope (wrong language, recommended exclusion, data-dominant):
    /// silently dropped, exactly as before suppressions existed.
    Drop,
}

/// Port of `_is_out_of_scope`, split so user-ignored files stay countable:
/// wrong language / recommended-set path → `Drop` (silent, as always); user
/// `.argotignore` match → `ScoreSuppressed`. Data-heavy files are NOT dropped
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
    patches: Vec<PatchBatch>,
    scorers: &mut HashMap<String, SequentialImportBpeScorer>,
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    slices: &HashMap<String, Vec<SliceEntry>>,
    new_file_thresholds: &HashMap<String, f64>,
    fit_corpus_files: &HashSet<String>,
    mute_rules: &[SuppressionRule],
    header_cpp: bool,
    stderr: &mut String,
) -> (Vec<Hit>, usize, Vec<FileScan>) {
    let mut hits: Vec<Hit> = Vec::new();
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

        // Inline suppression comments, parsed from the same content that gets
        // scored, with the language's own comment token.
        let inline = ext_to_lang(&ext)
            .and_then(|l| filter_adapters.get(l))
            .map(|a| parse_inline(&file_source, a.line_comment_prefix()))
            .unwrap_or_default();
        for w in &inline.warnings {
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
            let suppressed_by = if batch.ignored_by_pattern {
                Some(SuppressedBy::Exclude)
            } else if inline.suppresses(line, line_end, &reason) {
                Some(SuppressedBy::Inline)
            } else if mute_rules
                .iter()
                .any(|r| r.matches(&batch.file_path, &reason, &hash))
            {
                Some(SuppressedBy::Mute)
            } else {
                None
            };
            hits.push(Hit {
                score: scored.score,
                file_path: batch.file_path.clone(),
                line,
                line_end,
                source: batch.source.clone(),
                reason,
                flagged,
                threshold,
                hunk_content,
                evidence: scored.evidence,
                hash,
                suppressed_by,
                #[cfg(feature = "semantic")]
                semantic: None,
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

/// The architecture-graph pass — additive `Hit`s from the per-repo
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
) -> Vec<Hit> {
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
        // Fire if the added imports create a novel reversal/sink-out edge.
        let fired = graph
            .file_edges(&batch.file_path, &added)
            .iter()
            .any(|e| graph.classify(e).is_some());
        if !fired {
            continue;
        }
        let hunk_content = added.clone();
        let hash = hit_hash(&batch.file_path, "layering", &hunk_content);
        let inline = ext_to_lang(&extension(&batch.file_path))
            .and_then(|l| filter_adapters.get(l))
            .map(|a| parse_inline(&source, a.line_comment_prefix()));
        let suppressed_by = if inline
            .as_ref()
            .is_some_and(|i| i.suppresses(first_line, first_line, "layering"))
        {
            Some(SuppressedBy::Inline)
        } else if mute_rules
            .iter()
            .any(|r| r.matches(&batch.file_path, "layering", &hash))
        {
            Some(SuppressedBy::Mute)
        } else {
            None
        };
        hits.push(Hit {
            score: 1.0,
            file_path: batch.file_path.clone(),
            line: first_line,
            line_end: first_line,
            source: batch.source.clone(),
            reason: "layering".to_string(),
            flagged: true,
            threshold: 0.5,
            hunk_content,
            evidence: None,
            hash,
            suppressed_by,
            #[cfg(feature = "semantic")]
            semantic: None,
        });
    }
    hits
}

/// The semantic pass (F1 reinvention, F2 placement) — additive `Hit`s from
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
) -> Vec<Hit> {
    use crate::scoring::semantic::embedder::Embedder;
    use crate::scoring::semantic::index::{
        functions_in_file, FunctionRef, LoadedIndex, SemanticArtifact,
    };
    use crate::scoring::semantic::placement::PlacementScorer;
    use crate::scoring::semantic::redundant::RedundantScorer;
    use crate::scoring::semantic::SEMANTIC_INDEX_FILE;

    // Load the fit-time index artifact; its absence just means no semantic layer.
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

    // Load only the indices we actually need.
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

    // Acquire the embedder once; unavailable model → degrade (no semantic hits).
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

    // Embed all candidate functions in one batch.
    let texts: Vec<&str> = candidates.iter().map(|(_, _, f)| f.text.as_str()).collect();
    let vecs = match embedder.embed(&texts) {
        Ok(v) => v,
        Err(e) => {
            stderr.push_str(&format!("[argot] semantic embedding failed: {e}\n"));
            return Vec::new();
        }
    };

    // Dev-only feature capture (`ARGOT_SEM_DUMP=<path>`): append one JSON line
    // per candidate — its structural features, nearest neighbours and the fire
    // outcome — so bench sweeps can re-evaluate rule variants offline against a
    // saved index copy without re-running fit/check. Inert without the env var.
    let dump_path = std::env::var_os("ARGOT_SEM_DUMP");
    let mut dump_lines: Vec<String> = Vec::new();

    let mut hits = Vec::new();
    for ((bi, lang, f), vec) in candidates.iter().zip(&vecs) {
        let li = &loaded[lang];
        let batch = &patches[*bi];
        let mut fired: Option<&'static str> = None;
        // F1 first: a duplicate isn't "misplaced", it's "redundant" — the
        // stronger signal wins, one finding per function.
        if let Some(found) = RedundantScorer::new(&li.index, &li.reinvention).evaluate(f, vec) {
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
            if let Some(m) = PlacementScorer::new(&li.index, &li.placement).evaluate(f, vec) {
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

/// Build one semantic `Hit`, applying the mute + inline suppression
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
) -> Hit {
    let hunk_content = f.text.clone();
    let hash = hit_hash(&batch.file_path, reason, &hunk_content);
    let inline = ext_to_lang(&extension(&batch.file_path))
        .and_then(|l| filter_adapters.get(l))
        .map(|a| {
            let src = String::from_utf8_lossy(&batch.content);
            parse_inline(&src, a.line_comment_prefix())
        });
    let suppressed_by = if inline
        .as_ref()
        .is_some_and(|i| i.suppresses(f.line, f.end_line, reason))
    {
        Some(SuppressedBy::Inline)
    } else if mute_rules
        .iter()
        .any(|r| r.matches(&batch.file_path, reason, &hash))
    {
        Some(SuppressedBy::Mute)
    } else {
        None
    };
    Hit {
        score,
        file_path: batch.file_path.clone(),
        line: f.line,
        line_end: f.end_line,
        source: batch.source.clone(),
        reason: reason.to_string(),
        flagged: true,
        threshold,
        hunk_content,
        evidence: None,
        hash,
        suppressed_by,
        semantic: Some(sem),
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
    hits: &[&Hit],
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
    let mut file_hits: HashMap<String, Vec<&Hit>> = HashMap::new();
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

        let mut fhits: Vec<&Hit> = file_hits[fp].clone();
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

            // Per-reason evidence (names + `common here:`) sits between the
            // headline and the hunk body. `hunk_start_line = h.line` lets import
            // evidence render `(L7)` file-line annotations.
            if let Some(ev) = &h.evidence {
                for line in format_evidence(ev, use_color, h.line) {
                    out.push_str(&line);
                    out.push('\n');
                }
            }
            // Semantic findings render nearest-existing-code evidence (F4) — a
            // retrieval lookup, no LLM. Turns the statistic into "here's the
            // closest thing you already have."
            #[cfg(feature = "semantic")]
            if let Some(sem) = &h.semantic {
                for line in format_semantic_evidence(sem, use_color) {
                    out.push_str(&line);
                    out.push('\n');
                }
            }

            // Smart-peek keeps flagged lines in-frame; caret spans drive the
            // eslint-style `^^^^` underlines under the offending bytes.
            let must_show = evidence_lines_of_interest(h.evidence.as_ref());
            let caret_spans = evidence_caret_spans(h.evidence.as_ref());
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

/// The check exit code for the visible findings: 1 when any finding's rule is
/// configured `error` (or when `--error-on-warnings` promotes a warn-only
/// run), 0 otherwise. Unregistered reasons gate as `error` — a finding never
/// silently loses its gate.
fn gate_exit_code(visible: &[&Hit], settings: &RuleSettings, error_on_warnings: bool) -> i32 {
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
fn hit_records(hits: &[&Hit], settings: &RuleSettings) -> Vec<HitRecord> {
    hits.iter()
        .map(|h| {
            let evidence: Vec<String> = h
                .evidence
                .as_ref()
                .map(|ev| {
                    format_evidence(ev, false, h.line)
                        .into_iter()
                        .map(|l| l.trim().to_string())
                        .collect()
                })
                .unwrap_or_default();
            // Semantic findings carry their nearest-code evidence here too, so
            // JSON and SARIF consumers (GitHub code scanning) get it for free.
            // Rebind (not `mut`) so the base build stays warning-clean.
            #[cfg(feature = "semantic")]
            let evidence = match &h.semantic {
                Some(sem) => format_semantic_evidence(sem, false)
                    .into_iter()
                    .map(|l| l.trim().to_string())
                    .collect(),
                None => evidence,
            };
            HitRecord {
                path: h.file_path.clone(),
                line_start: h.line,
                line_end: h.line_end,
                score: h.score,
                threshold: h.threshold,
                confidence: confidence(&h.reason, h.score, h.threshold).to_string(),
                severity: settings.severity_of_reason(&h.reason).as_str().to_string(),
                rule: rules::code_for_reason(&h.reason).to_string(),
                rule_label: rules::label_for_reason(&h.reason).to_string(),
                source: h.source.clone(),
                hash: h.hash.clone(),
                evidence,
            }
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
        _ => render_json(meta, records),
    }
}

/// Commits between the fit SHA and HEAD to trigger the freshness warning.
const FRESHNESS_WARN_COMMITS: usize = 10;

/// How many commits HEAD is ahead of the fit SHA (`None` when either end
/// cannot be resolved — shallow clones, rewritten history, detached states
/// must never break check).
fn commits_since_fit(repo_path: &str, fit_sha: &str) -> Option<usize> {
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
    let config = ArgotConfig::load(Path::new(&args.repo_path));
    // Effective per-rule severities: defaults ⊕ [rules] ⊕ CLI --rule overrides.
    let settings = config.rule_settings(&args.rule_overrides);

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

    let mut stderr = String::new();

    // Name the model that judged this diff — reproducibility + "is my model the
    // same as my colleague's?". On stderr (human) so stdout stays byte-parity;
    // machine formats carry it in the report meta instead.
    if !args.format.is_machine() {
        stderr.push_str(&format!("[argot] model: {model_hash}\n"));
    }

    // Freshness: a stale model turns ordinary drift into noise (a month of
    // drift on a busy workspace measured ~14× the hit volume of a fresh
    // fit). Warn when HEAD has moved substantially since the fit.
    if let Some(fit_sha) = &fit_sha {
        if let Some(behind) = commits_since_fit(&args.repo_path, fit_sha) {
            if behind >= FRESHNESS_WARN_COMMITS {
                stderr.push_str(&format!(
                    "[argot] model fitted {behind} commits ago — voice may have drifted; re-run `argot fit`\n"
                ));
            }
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

    // Additive semantic pass over the same scoped batches (borrowed before
    // score_patches consumes them). Produces reinvention/placement hits; a
    // no-op without the feature or when the index/model is unavailable.
    #[cfg(feature = "semantic")]
    let semantic_extra = if settings.group_enabled(rules::GROUP_SEMANTIC) {
        semantic_hits(
            &filtered,
            &args.argot_dir,
            &filter_adapters,
            &mutes.active,
            &config.detect,
            header_cpp,
            &mut stderr,
        )
    } else {
        // Both semantic rules are off: no index load, no model, no cost.
        Vec::new()
    };

    // Compute arch hits before `filtered` is moved into `score_patches`.
    #[cfg(feature = "arch")]
    let arch_extra = if settings.severity_of_reason("layering") != RuleSeverity::Off {
        arch_hits(
            &filtered,
            &args.argot_dir,
            &filter_adapters,
            &mutes.active,
            &mut stderr,
        )
    } else {
        Vec::new()
    };

    let (hits, hunk_count, files_scanned) = score_patches(
        filtered,
        &mut scorers,
        &filter_adapters,
        &slices,
        &new_file_thresholds,
        &fit_corpus_files,
        &mutes.active,
        header_cpp,
        &mut stderr,
    );

    // Merge the semantic hits (rebind rather than `mut` so the base build has
    // no unused-mut and stays byte-for-byte identical).
    #[cfg(feature = "semantic")]
    let hits = {
        let mut hits = hits;
        hits.extend(semantic_extra);
        hits
    };

    // Merge the architecture-graph hits (same rebind discipline).
    #[cfg(feature = "arch")]
    let hits = {
        let mut hits = hits;
        hits.extend(arch_extra);
        hits
    };

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
    let above_all: Vec<&Hit> = if let Some(t) = threshold_override {
        hits.iter().filter(|h| h.score >= t).collect()
    } else {
        hits.iter().filter(|h| h.flagged).collect()
    };

    // Suppressed ≠ deleted: drop muted hits from output and exit-code
    // consideration, but say how many were muted (and by which surface).
    let (above, suppressed): (Vec<&Hit>, Vec<&Hit>) = above_all
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
    let visible: Vec<&Hit> = above
        .iter()
        .copied()
        .filter(|h| {
            let t = threshold_override.unwrap_or(h.threshold);
            confidence_index(confidence(&h.reason, h.score, t)) >= min_idx
        })
        .collect();

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
        let records = hit_records(&visible, &settings);
        let meta = report_meta(&args, scan_label, hunk_count, files_scanned, &model_hash);
        return CheckOutcome {
            stdout: render_machine(args.format, &meta, &records),
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
}
