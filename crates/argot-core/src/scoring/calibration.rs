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
use crate::scoring::adapters::{Language, LanguageAdapter};
use crate::scoring::bpe_scorer::BpeScorer;
use crate::scoring::call_receiver::CallReceiverScorer;
use crate::scoring::conventions::{fit_convention_frequencies, ConventionScorer};
use crate::scoring::model::{ConventionModel, LanguageModel};
use crate::scoring::typicality::TypicalityModel;
use crate::suppress::PathSuppressions;
use crate::text::{read_text_lossy, splitlines, splitlines_keepends, universal_newlines};
use anyhow::{bail, Result};
use md5::{Digest, Md5};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

const MIN_BODY_LINES: usize = 5;
/// Window length (lines) for calibrating the convention identifier-shape bar
/// over diff-hunk-sized sub-regions of each candidate declaration rather than
/// the whole declaration — the unit check actually scores. Sized to a typical
/// diff hunk; declarations shorter than this are scored whole.
const CONVENTION_BAR_WINDOW_LINES: usize = 8;
/// v3: adds the per-language `model` block (fit-time BPE stats + callee
/// attestation snapshot) and repo-owned import modules. Check refuses other
/// versions — regenerate via `argot fit`.
const CONFIG_VERSION: u32 = 3;
/// Schema version of `.argot/manifest.json` — bumped independently of
/// `CONFIG_VERSION` so the artifact contract can evolve on its own cadence.
pub const MANIFEST_VERSION: u32 = 1;

/// Name of the model manifest inside `.argot/`.
pub const MANIFEST_FILE: &str = "manifest.json";

/// First 12 hex chars of the MD5 of `bytes` — the repo-wide short-hash idiom
/// (`model_hash`, config hash, etc.).
pub fn short_hash(bytes: &[u8]) -> String {
    let digest = Md5::new().chain_update(bytes).finalize();
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    hex[..12].to_string()
}

/// Combine per-language model fingerprints into one stable overall hash. The
/// input is sorted by language name, so the result is order-independent and
/// identical for identical models. A single-language repo still gets a distinct
/// combined hash (not the raw per-language one) so the two can't be confused.
pub fn combined_model_hash(per_language: &BTreeMap<String, String>) -> String {
    let mut buf = String::new();
    for (lang, hash) in per_language {
        buf.push_str(lang);
        buf.push(':');
        buf.push_str(hash);
        buf.push('\n');
    }
    short_hash(buf.as_bytes())
}

/// The inspectable model artifact (`.argot/manifest.json`): a stable fingerprint
/// of what argot learned, so two fits of the same corpus+config are provably
/// identical and a stale or foreign artifact is obvious at a glance.
#[derive(Serialize)]
struct Manifest {
    manifest_version: u32,
    config_version: u32,
    /// Combined fingerprint of every language's fit-time model snapshot.
    model_hash: String,
    /// Fingerprint of the emitted `scorer-config.json` bytes.
    scorer_config_hash: String,
    /// Repo HEAD sha when the model was fitted (`unknown` outside a git repo).
    fit_commit_sha: String,
    fit_timestamp: String,
    corpus: CorpusSummary,
    languages: Vec<LangSummary>,
}

#[derive(Serialize)]
struct CorpusSummary {
    files: usize,
    lines: usize,
}

#[derive(Serialize)]
struct LangSummary {
    language: String,
    threshold: f64,
    model_hash: String,
    n_cal: usize,
    files: usize,
}

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
/// symmetric. It was gated off when a forced cluster-rare rule made the
/// catalog-mode false-positive control regress; in production the rare rule is
/// auto-detected per corpus, and the production-path FP controls re-validated
/// this setting.
const CR_PARSE_ERROR_FALLBACK: bool = true;
/// Score added when a hunk's rarest present convention clears its calibrated
/// bar (same magnitude as the cluster bonus).
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

/// Which language this repo's ambiguous `.h` headers belong to. `.h` is used by
/// both C and C++; a header-only C++ library keeps its logic in `.h` with the
/// translation units in `.cc`/`.cpp`, so filing every `.h` under C (the naive
/// default) starves the C++ model and mis-scores the bulk of the code. Decide
/// per repo by translation-unit majority — `.cpp`/`.cc`/`.cxx` (C++) vs `.c`
/// (C). Computed identically wherever the pipeline classifies a `.h` (extract,
/// calibrate, check) so the stages stay in lock-step.
pub fn header_is_cpp(source_dir: &Path) -> bool {
    fn walk(dir: &Path, root: &Path, c: &mut usize, cpp: &mut usize) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(t) if t.is_dir() => {
                    let name = basename(&path);
                    // Speed prune: never any authored voice in these; the
                    // per-file `is_excluded_path` below is the correctness gate.
                    if matches!(name.as_str(), ".git" | "node_modules" | "target" | "vendor") {
                        continue;
                    }
                    walk(&path, root, c, cpp);
                }
                Ok(t) if t.is_file() => {
                    if is_excluded_path(&path, root) {
                        continue;
                    }
                    let name = basename(&path);
                    if name.ends_with(".c") {
                        *c += 1;
                    } else if name.ends_with(".cpp")
                        || name.ends_with(".cc")
                        || name.ends_with(".cxx")
                    {
                        *cpp += 1;
                    }
                }
                _ => {}
            }
        }
    }
    let (mut c, mut cpp) = (0usize, 0usize);
    walk(source_dir, source_dir, &mut c, &mut cpp);
    cpp > c
}

/// [`language_for_filename`], but resolving the C/C++ `.h` ambiguity with a
/// repo-level `header_is_cpp` decision so all stages agree.
pub fn language_for_filename_ctx(name: &str, header_is_cpp: bool) -> Option<Language> {
    match (language_for_filename(name), header_is_cpp) {
        (Some(Language::C), true) if name.ends_with(".h") => Some(Language::Cpp),
        (other, _) => other,
    }
}

/// Reads source content the way a fit *should* see it: from the committed HEAD
/// blob, not the working tree. On a clean checkout the two are byte-identical
/// (both normalize newlines via [`universal_newlines`]); on a dirty tree this
/// ignores uncommitted edits, so foreign code an agent just wrote isn't
/// laundered into the learned voice. Files absent from HEAD (untracked / newly
/// added) resolve to `None` — they're uncommitted, so calibration excludes
/// them. Outside a git repo, or on an unborn HEAD, every read falls back to the
/// working-tree file, preserving today's behaviour for non-repo callers.
struct HeadSource {
    // (repo, relpath → blob oid, workdir). `None` → always fall back to disk.
    inner: Option<(
        git2::Repository,
        std::collections::HashMap<String, git2::Oid>,
        PathBuf,
    )>,
}

impl HeadSource {
    fn new(dir: &Path) -> Self {
        let Ok(repo) = crate::git_walk::open_repo(&dir.to_string_lossy()) else {
            return Self { inner: None };
        };
        // Canonicalize the workdir so it compares equal to canonicalized file
        // paths in `read` — macOS symlinks the temp/`/var` root to `/private/var`,
        // and git2 reports the resolved form, so a raw prefix check would miss.
        let Some(workdir) = repo
            .workdir()
            .map(|w| std::fs::canonicalize(w).unwrap_or_else(|_| w.to_path_buf()))
        else {
            return Self { inner: None };
        };
        let mut blobs: std::collections::HashMap<String, git2::Oid> =
            std::collections::HashMap::new();
        {
            let Ok(head) = repo.head() else {
                return Self { inner: None };
            };
            let Ok(tree) = head.peel_to_tree() else {
                return Self { inner: None };
            };
            let _ = tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
                if entry.kind() == Some(git2::ObjectType::Blob) {
                    if let Some(name) = entry.name() {
                        blobs.insert(format!("{root}{name}"), entry.id());
                    }
                }
                0 // continue the walk
            });
        }
        Self {
            inner: Some((repo, blobs, workdir)),
        }
    }

    /// Source text for `path` as a fit should see it. A **tracked** file resolves
    /// to its committed HEAD blob — so a working-tree modification (an agent's
    /// just-added foreign import in an existing file) is *not* learned. Anything
    /// not in HEAD (a new/untracked file, or a path outside this repo) is read
    /// as-is from disk, so nested and non-repo callers behave exactly as before.
    /// Byte-identical to a working-tree read on a clean checkout (both normalize
    /// newlines). `None` only when the disk read itself fails.
    fn read(&self, path: &Path) -> Option<String> {
        if let Some((repo, blobs, workdir)) = &self.inner {
            // Match the canonicalized workdir (resolves the `/var` symlink etc.).
            let canon = std::fs::canonicalize(path);
            let lookup = canon.as_deref().unwrap_or(path);
            if let Ok(rel) = lookup.strip_prefix(workdir) {
                let rel = rel.to_string_lossy().replace('\\', "/");
                if let Some(oid) = blobs.get(&rel) {
                    if let Ok(blob) = repo.find_blob(*oid) {
                        return Some(universal_newlines(&String::from_utf8_lossy(blob.content())));
                    }
                }
            }
        }
        read_text_lossy(path).ok()
    }
}

/// A calibration candidate: hunk text + originating file path + file source.
/// Line bounds are 1-indexed inclusive within `file_source` and back the
/// parse-error host fallback for callee extraction.
#[derive(Clone)]
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
    collect_candidates_with(
        source_dir,
        adapter,
        &PathSuppressions::recommended(),
        &crate::config::DetectConfig::default(),
    )
}

/// [`collect_candidates`] against a fully resolved path-suppression set
/// (recommended built-ins + `.argotignore`). Calibration sampling, the
/// check-time scope filter, and `argot inspect` all consult the same
/// [`PathSuppressions`] so their scopes stay in lock-step.
pub fn collect_candidates_with(
    source_dir: &Path,
    adapter: &dyn LanguageAdapter,
    path_suppressions: &PathSuppressions,
    detect: &crate::config::DetectConfig,
) -> Vec<Candidate> {
    // `.h` routes to whichever of C / C++ this repo predominantly is, so a
    // header-only C++ library's headers calibrate under the C++ model, not C.
    let exts: Vec<&str> = match adapter.language() {
        Language::Python => vec![".py"],
        Language::Typescript => vec![".ts", ".tsx"],
        Language::Javascript => vec![".js", ".jsx"],
        Language::Go => vec![".go"],
        Language::Rust => vec![".rs"],
        Language::C if header_is_cpp(source_dir) => vec![".c"],
        Language::C => vec![".c", ".h"],
        Language::Java => vec![".java"],
        Language::CSharp => vec![".cs"],
        Language::Php => vec![".php"],
        Language::Cpp if header_is_cpp(source_dir) => vec![".cpp", ".cc", ".hpp", ".cxx", ".h"],
        Language::Cpp => vec![".cpp", ".cc", ".hpp", ".cxx"],
        Language::Ruby => vec![".rb"],
    };
    let head = HeadSource::new(source_dir);
    let mut out = Vec::new();
    for &ext in &exts {
        for src_file in rglob_sorted(source_dir, ext) {
            if path_suppressions.is_suppressed_abs(&src_file, source_dir) {
                continue;
            }
            let source = match head.read(&src_file) {
                Some(s) => s,
                None => continue,
            };
            // Generated code (transpiled JS, protobuf stubs, `// Code generated`)
            // is not authored voice — exclude it from calibration, matching what
            // `inspect` reports and how `check` skips it.
            if adapter.is_data_dominant(&source, detect.data_threshold)
                || adapter.is_auto_generated(&source, &detect.generated_markers)
            {
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
    /// Honest threshold for hunks in files absent from the fit corpus. A new
    /// file gets full unattested-callee (alpha) mass with no cluster routing —
    /// a systematically higher score distribution than an edit to an existing
    /// file — so it needs its own, higher bar (issue #92 new-file flooding).
    /// `check` applies it only to new-file hunks; existing files keep
    /// `threshold`. Never below `threshold`.
    new_file_threshold: f64,
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
    /// Optional per-slice thresholds (per-subdirectory / per-author voice).
    /// Omitted entirely for an unsliced fit, so those configs are byte-identical
    /// to before this field existed.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    slices: Vec<SliceConfig>,
    /// Fit-time model snapshot: BPE token stats + callee attestation +
    /// cluster partition. Check scores against this, never the live tree.
    model: LanguageModel,
}

/// One calibrated slice: its own threshold applies to hunks whose repo-relative
/// path matches any of `paths` (a top-level-dir prefix, an explicit glob-free
/// prefix, or the exact files an author owns).
#[derive(Serialize, Clone)]
struct SliceConfig {
    name: String,
    paths: Vec<String>,
    threshold: f64,
}

/// A slice resolved to the repo-relative path prefixes/files its threshold
/// covers.
#[derive(Clone)]
pub struct ResolvedSlice {
    pub name: String,
    pub paths: Vec<String>,
}

/// True when `rel_path` (repo-relative, `/`-separated) falls in the slice: it
/// starts with one of the slice's prefixes, or equals one of its files.
fn slice_matches(rel_path: &str, paths: &[String]) -> bool {
    paths
        .iter()
        .any(|p| rel_path == p || rel_path.starts_with(p))
}

/// Repo-relative, `/`-separated form of `path` (best-effort: strips the repo
/// prefix when present, and normalizes Windows separators).
fn rel_to_repo(path: &Path, repo_dir: &Path) -> String {
    let rel = path.strip_prefix(repo_dir).unwrap_or(path);
    rel.to_string_lossy().replace('\\', "/")
}

/// Resolve raw `--slice` specs (`path:<prefix>`, `author:<email>`, `auto`) into
/// concrete slices over `corpus_files` (repo-relative paths). Unknown/empty
/// specs are dropped. `auto` expands to one slice per top-level directory that
/// holds at least [`SLICE_AUTO_MIN_FILES`] source files.
pub fn resolve_slices(
    repo_dir: &Path,
    corpus_rel_files: &[String],
    raw: &[String],
) -> Vec<ResolvedSlice> {
    let mut out = Vec::new();
    for spec in raw {
        let spec = spec.trim();
        if spec == "auto" {
            out.extend(auto_slices(corpus_rel_files));
        } else if let Some(prefix) = spec.strip_prefix("path:") {
            let prefix = prefix.trim().to_string();
            if !prefix.is_empty() {
                out.push(ResolvedSlice {
                    name: format!("path:{prefix}"),
                    paths: vec![prefix],
                });
            }
        } else if let Some(email) = spec.strip_prefix("author:") {
            let email = email.trim();
            let files = author_files(repo_dir, email);
            if !files.is_empty() {
                out.push(ResolvedSlice {
                    name: format!("author:{email}"),
                    paths: files,
                });
            }
        }
    }
    out
}

/// A top-level directory needs this many source files to become an auto slice.
const SLICE_AUTO_MIN_FILES: usize = 10;

/// A slice needs at least this many calibration candidates to get its own
/// threshold; below it, the whole-repo threshold is more stable.
const SLICE_MIN_CANDIDATES: usize = 20;

fn auto_slices(corpus_rel_files: &[String]) -> Vec<ResolvedSlice> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for f in corpus_rel_files {
        if let Some(idx) = f.find('/') {
            *counts.entry(f[..idx].to_string()).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .filter(|(_, n)| *n >= SLICE_AUTO_MIN_FILES)
        .map(|(dir, _)| ResolvedSlice {
            name: format!("path:{dir}/"),
            paths: vec![format!("{dir}/")],
        })
        .collect()
}

/// Repo-relative files an author has touched (in-process via libgit2, so no
/// external `git`). Empty when the repo can't be opened or the author has no
/// commits — the caller then drops the slice.
fn author_files(repo_dir: &Path, email: &str) -> Vec<String> {
    let Ok(repo) = git2::Repository::open(repo_dir) else {
        return Vec::new();
    };
    let Ok(mut walk) = repo.revwalk() else {
        return Vec::new();
    };
    if walk.push_head().is_err() {
        return Vec::new();
    }
    let mut files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for oid in walk.flatten() {
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };
        if commit.author().email() != Some(email) {
            continue;
        }
        if commit.parent_count() != 1 {
            continue;
        }
        let (Ok(tree), Ok(parent)) = (commit.tree(), commit.parent(0)) else {
            continue;
        };
        let Ok(parent_tree) = parent.tree() else {
            continue;
        };
        if let Ok(diff) = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None) {
            for delta in diff.deltas() {
                if let Some(p) = delta.new_file().path().and_then(|p| p.to_str()) {
                    files.insert(p.to_string());
                }
            }
        }
    }
    files.into_iter().collect()
}

#[derive(Serialize)]
struct ScorerConfig {
    version: u32,
    languages: BTreeMap<String, LangConfig>,
    /// Every fit-corpus file, repo-relative — the authoritative set `check` uses
    /// to tell a new file (absent here → new-file threshold) from an edit to a
    /// known one. Includes data-dominant files (which are filtered out of
    /// clustering), so a fixture/edit in a data-heavy known file is NOT
    /// misclassified as new (issue #92). Omitted for configs predating the
    /// field — then `check` falls back to cluster membership.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    corpus_files: Vec<String>,
}

fn adapter_for(language: Language) -> Box<dyn LanguageAdapter> {
    match language {
        Language::Python => Box::new(PythonAdapter::new()),
        Language::Typescript => Box::new(TypeScriptAdapter::new()),
        Language::Javascript => Box::new(JavaScriptAdapter::new()),
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

/// Canonical config-key name for a scoring language.
/// Public so `inspect` reports under the same keys `scorer-config.json` uses.
pub fn language_name(language: Language) -> &'static str {
    match language {
        Language::Python => "python",
        Language::Typescript => "typescript",
        Language::Javascript => "javascript",
        Language::Go => "go",
        Language::Rust => "rust",
        Language::C => "c",
        Language::Java => "java",
        Language::CSharp => "csharp",
        Language::Php => "php",
        Language::Cpp => "cpp",
        Language::Ruby => "ruby",
    }
}

/// Extension → language routing used to partition the corpus (`.py` → python;
/// `.ts`/`.tsx` → typescript; `.js`/`.jsx` → javascript; `.cs` → csharp).
/// Public so `inspect` classifies files with exactly the calibration routing.
pub fn language_for_filename(name: &str) -> Option<Language> {
    let ext = match name.rfind('.') {
        Some(i) => &name[i..],
        None => return None,
    };
    match ext {
        ".py" => Some(Language::Python),
        ".ts" | ".tsx" => Some(Language::Typescript),
        ".js" | ".jsx" => Some(Language::Javascript),
        ".go" => Some(Language::Go),
        ".rs" => Some(Language::Rust),
        ".c" | ".h" => Some(Language::C),
        ".java" => Some(Language::Java),
        ".cs" => Some(Language::CSharp),
        ".php" => Some(Language::Php),
        ".cpp" | ".cc" | ".hpp" | ".cxx" => Some(Language::Cpp),
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

/// Per-file BPE token counts for leave-one-file-out calibration, keyed by
/// the corpus file paths [`Candidate::file_path`] resolves to.
pub type PerFileTokenCounts =
    std::collections::HashMap<PathBuf, std::collections::HashMap<u32, u64>>;

/// Per-seed calibration thresholds: for each seed, `max` over sampled
/// cal-hunk scores (BPE + cluster contribution at alpha/root_bonus 0).
///
/// The BPE side of each cal hunk is scored **leave-one-file-out** when
/// `per_file_counts` is given: the hunk's own file's token counts are
/// subtracted from the repo distribution, so the hunk is scored the way
/// check scores code the model has not memorized. Calibrating on memorized
/// scores deflates the threshold below the level genuinely-unseen idiomatic
/// code reaches — the honest-FP flood of issue #92.
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
    per_file_counts: Option<&PerFileTokenCounts>,
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
            let blanked = blank_prose_lines(&c.hunk, &prose);
            let raw_bpe = match per_file_counts.and_then(|m| m.get(&c.file_path)) {
                Some(counts) => bpe.bpe_score_excluding(&blanked, counts),
                None => bpe.bpe_score(&blanked),
            };
            // Cal side scores without local-binding attestation: candidates
            // are corpus files whose callees are attested anyway, so the
            // omission only leaves the threshold marginally conservative.
            // The call-receiver side stays memorized on purpose: cluster
            // branches only fire on files the fit clustered (new files are
            // not cluster-routed at check time), so memorized cal is already
            // symmetric with check for them — only the BPE side had the
            // train-on-test leak (issue #92).
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

/// Per-seed **new-file** thresholds: like [`multi_seed_thresholds`], but each
/// cal hunk is scored as though its file had just been added post-fit —
/// BPE leave-one-file-out (as before) plus a call-receiver contribution from
/// [`CallReceiverScorer::weighted_contribution_as_new`] (cluster off, the real
/// check-time `alpha`/`root_bonus`, attestation requiring `df ≥ 2`). The main
/// threshold zeroes `alpha` and keeps cluster routing on, which models an edit
/// to an *existing* file; a new file gets no cluster mass but full alpha mass on
/// its unattested callees, landing systematically higher. This threshold is the
/// honest operating point for that distribution, applied by `check` only to
/// hunks whose file was absent from the fit corpus (issue #92 new-file flood).
#[allow(clippy::too_many_arguments)]
pub fn multi_seed_new_file_thresholds(
    candidates: &[Candidate],
    bpe: &BpeScorer,
    per_file_counts: Option<&PerFileTokenCounts>,
    call_receiver: &CallReceiverScorer,
    adapter: &dyn LanguageAdapter,
    typicality: &TypicalityModel,
    cfg: &ThresholdRunConfig,
    alpha: f64,
    root_bonus: f64,
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
            let blanked = blank_prose_lines(&c.hunk, &prose);
            let raw_bpe = match per_file_counts.and_then(|m| m.get(&c.file_path)) {
                Some(counts) => bpe.bpe_score_excluding(&blanked, counts),
                None => bpe.bpe_score(&blanked),
            };
            let contrib = call_receiver.weighted_contribution_as_new(
                &c.hunk,
                Some(&c.file_path),
                alpha,
                root_bonus,
                cfg.cluster_bonus,
                cfg.cap,
                Some((&c.file_source, c.hunk_start_line, c.hunk_end_line)),
                &Default::default(),
            );
            cal_scores.push(raw_bpe + contrib);
        }
        let t = cal_scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        seed_thresholds.push(if t.is_finite() { t } else { 0.0 });
    }
    seed_thresholds
}

/// Options for `run_calibrate` (defaults mirror the Python CLI, including the
/// asymmetric-calibration knobs the final Python calibrator shipped).
pub struct CalibrateOptions {
    pub n_cal: usize,
    pub seed: u64,
    pub n_seeds: usize,
    pub evidence_top_n: usize,
    pub repo_sha: String,
    pub timestamp_utc: String,
    /// Cluster-rare threshold for the CHECK-time scorer: a callee attested in
    /// ≤ N cluster files is treated as cluster-absent. 0 disables the rule
    /// (baseline). Calibration itself always runs with the rule off
    /// (asymmetric calibration — see docs/agents/calibration-contract.md).
    pub cluster_rare_threshold: usize,
    /// Minimum cluster size for the rare rule to fire.
    pub cluster_size_min: usize,
    /// Per-corpus auto-detect: probe the calibration distribution's rare-rule
    /// fire rate; keep the rule when it is discriminative (fire rate below
    /// `asym_fire_rate_threshold`), disable it when noisy (would FP-flood).
    pub auto_select_asym_cal: bool,
    pub asym_fire_rate_threshold: f64,
    /// Raw `--slice` specs (`path:<prefix>`, `author:<email>`, `auto`). Each
    /// resolved slice gets its own calibrated threshold, dispatched by file path
    /// at check time. Empty = a single whole-repo threshold (today's behaviour).
    pub slices: Vec<String>,
    /// Fit and emit the convention-rarity stage (syntax/identifier-shape
    /// surprisal). Off by default and NOT a user-facing knob — production
    /// `fit`/`check` never expose it. It is *secondary coverage* (never gated —
    /// see `benchmarks/catalogs/RUBRIC.md`) and a co-headline false-alarm
    /// driver whose feature space overlaps in-voice code (jellyfin holdout:
    /// FP fire at the same convention-bar ratios as the two catches it carries,
    /// so no threshold separates them). The benchmark harness sets this to
    /// measure the with/without trade-off; nothing else should.
    pub enable_conventions: bool,
}

impl Default for CalibrateOptions {
    fn default() -> Self {
        Self {
            // n_cal=100 × 7 seeds is the configuration the bench has
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
            slices: Vec::new(),
            enable_conventions: false,
        }
    }
}

/// Calibrate the convention firing bars over ALL candidates (not the threshold's
/// `n_cal` sample): the bar is a max-gate, so sampling only lowers it and fires
/// the stage on ordinary code. Shared by the production calibrator and the
/// benchmark harness so the two can't drift.
///
/// The **syntax** bar is a single max over whole declarations (it reads the
/// parsed AST; windowing would re-parse the host per window, and node-kind mix
/// doesn't concentrate in sub-regions). The **identifier** bar is *per
/// morphology*, taken over diff-hunk-sized windows — the unit check actually
/// scores. Two reasons:
/// - **Windowing:** a whole declaration averages its identifier mix and never
///   reaches the skew of one sub-region (a fluent camelCase chain, a
///   `SCREAMING_SNAKE` block), so a later commit touching a nearby line would
///   re-score in-voice code above a bar its own repo never set. The shape
///   feature is a byte scan, so windowing is cheap.
/// - **Per-shape:** a single scalar bar let an in-voice concentrated shape gate a
///   genuinely-foreign one (camelCase in a snake_case repo). Each shape's bar is
///   the most-skewed window the repo's own code contains *for that shape*.
pub fn calibrate_convention_bars(
    candidates: &[Candidate],
    convention_model: &ConventionModel,
    language: Language,
    typicality: &TypicalityModel,
) -> (f64, BTreeMap<String, f64>) {
    let conv = ConventionScorer::new(convention_model.clone(), language);
    let mut syntax_bar = 0.0f64;
    let mut ident_bars: BTreeMap<String, f64> = BTreeMap::new();
    let raise = |surps: &BTreeMap<String, f64>, bars: &mut BTreeMap<String, f64>| {
        for (shape, &s) in surps {
            let e = bars.entry(shape.clone()).or_insert(0.0);
            *e = e.max(s);
        }
    };
    for c in candidates {
        if typicality.is_atypical(&c.hunk).0 {
            continue;
        }
        let scores = conv.scores(
            &c.hunk,
            Some((&c.file_source, c.hunk_start_line, c.hunk_end_line)),
        );
        syntax_bar = syntax_bar.max(scores.syntax_surprisal);
        raise(&scores.ident_surprisals, &mut ident_bars);

        let lines = splitlines(&c.file_source);
        let start0 = c.hunk_start_line.saturating_sub(1);
        let end0 = c.hunk_end_line.min(lines.len());
        let span = end0.saturating_sub(start0);
        if span == 0 {
            continue;
        }
        let win = CONVENTION_BAR_WINDOW_LINES.min(span);
        let last_start = end0 - win;
        let mut ws = start0;
        loop {
            let we = ws + win;
            raise(
                &conv.ident_surprisals(&lines[ws..we].join("\n")),
                &mut ident_bars,
            );
            if ws >= last_start {
                break;
            }
            ws += 1;
        }
    }
    (syntax_bar, ident_bars)
}

/// Run calibration and write `scorer-config.json` to `output`.
///
/// `repo_dir` is the target repo (candidate rglob source). `repo_corpus_path`
/// lists corpus files (from `train`). `generic_baseline_json` is the embedded
/// baseline bytes.
/// Write a fit artifact via temp-file + rename, so a fit killed mid-write (a
/// laptop shutdown, a background refit reaped by the OS) can never leave a
/// half-written artifact behind — the previous version survives intact.
fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

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

    // Partition corpus by language (routing `.h` per the repo's C/C++ majority).
    let header_cpp = header_is_cpp(repo_dir);
    let mut by_lang: BTreeMap<&'static str, (Language, Vec<PathBuf>)> = BTreeMap::new();
    for f in &corpus_files {
        if let Some(lang) = language_for_filename_ctx(&basename(f), header_cpp) {
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

    // Resolve `--slice` specs to concrete path sets once (cross-language). Each
    // language then calibrates its own threshold for each slice's candidates.
    let corpus_rel: Vec<String> = corpus_files
        .iter()
        .map(|p| rel_to_repo(p, repo_dir))
        .collect();
    let resolved_slices = resolve_slices(repo_dir, &corpus_rel, &opts.slices);

    let mut languages: BTreeMap<String, LangConfig> = BTreeMap::new();
    let mut thresholds_out: Vec<(String, f64)> = Vec::new();
    // Per-language corpus sizes for the manifest (files scored, source lines).
    let mut per_lang_files: BTreeMap<String, usize> = BTreeMap::new();
    let mut total_lines: usize = 0;

    // Resolved config: the path-suppression set (recommended built-ins +
    // `[exclude].paths`) and the `[detect]` heuristics — the same values `check`
    // consults (lock-step principle).
    let config = crate::config::ArgotConfig::load(repo_dir);
    let path_suppressions = config.path_suppressions();
    let detect = &config.detect;
    // Captured now: `config` (the ArgotConfig) is shadowed by the ScorerConfig
    // later; the health artifact records which configuration this fit reflects.
    let config_fingerprint_at_fit = crate::health::config_fingerprint(&config);
    // Effective [rules] severities — a group turned off in argot.toml skips
    // its whole fit-time artifact (semantic index / layering graph) and cost.
    // Only the feature-gated layers read it.
    #[cfg(any(feature = "semantic", feature = "arch", feature = "integrity"))]
    let rule_settings = config.rule_settings(&Vec::new());
    // Fit from committed HEAD, not the working tree — an uncommitted foreign
    // edit must not be learned as part of the voice it's about to be checked
    // against. Byte-identical to a working-tree read on a clean checkout.
    let head = HeadSource::new(repo_dir);

    // Semantic layer: acquire the embedder once for the whole fit (lazy model
    // load / one-time fetch). `None` when the model is unavailable (offline) —
    // the fit still produces the full statistical model, just no semantic index.
    #[cfg(feature = "semantic")]
    let embedder = if !rule_settings.group_enabled(crate::rules::GROUP_SEMANTIC) {
        // Both semantic rules are off in [rules]: no model, no index, no cost.
        eprintln!("argot: semantic rules are off in [rules] — skipping the semantic index");
        None
    } else {
        match crate::scoring::semantic::embedder::Embedder::ready() {
            Ok(e) => e,
            Err(e) => {
                // Degrade to no-semantic-index, but NEVER silently: a load failure
                // (GPU memory pressure, corrupt model) must be visible, or a bench
                // records "0 recall" where the truth is "no embedder".
                eprintln!("argot: semantic model failed to load — skipping semantic index: {e:#}");
                None
            }
        }
    };
    #[cfg(feature = "semantic")]
    let mut semantic_artifact =
        crate::scoring::semantic::index::SemanticArtifact::new(opts.repo_sha.clone());
    // The previous fit's index, when it exists and was built by THIS model:
    // unchanged functions reuse their vectors (incremental refresh), so a
    // routine refit embeds only what actually changed. A stale/other-model
    // artifact yields no reuse — full re-embed, never mixed spaces.
    #[cfg(feature = "semantic")]
    let prior_artifact = std::fs::read_to_string(
        output.with_file_name(crate::scoring::semantic::SEMANTIC_INDEX_FILE),
    )
    .ok()
    .and_then(|raw| crate::scoring::semantic::index::SemanticArtifact::from_json_str(&raw).ok())
    .filter(|a| a.validate_current().is_ok());

    for (name, (language, lang_files)) in by_lang {
        let adapter = adapter_for(language);

        // Read corpus sources once (shared by BPE + call-receiver + evidence).
        let repo_files: Vec<(PathBuf, String)> = lang_files
            .iter()
            .filter_map(|p| head.read(p).map(|s| (p.clone(), s)))
            .collect();
        // Exclude data-dominant AND auto-generated files (transpiled output,
        // generated stubs) — the model learns authored voice only.
        let filtered: Vec<(PathBuf, String)> = repo_files
            .iter()
            .filter(|(_, s)| {
                !adapter.is_data_dominant(s, detect.data_threshold)
                    && !adapter.is_auto_generated(s, &detect.generated_markers)
            })
            .cloned()
            .collect();
        // No authored voice for this language — every file is data-dominant or
        // generated (e.g. a TS repo's transpiled `.js`). Emit no model rather
        // than fall back to learning generated code as the repo's voice.
        if filtered.is_empty() {
            continue;
        }
        let corpus = &filtered;
        let sources: Vec<String> = corpus.iter().map(|(_, s)| s.clone()).collect();
        per_lang_files.insert(name.to_string(), corpus.len());
        total_lines += sources.iter().map(|s| s.lines().count()).sum::<usize>();

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
            detect.data_threshold,
        )
        .map_err(anyhow::Error::msg)?;

        // Candidates for sampling.
        let candidates =
            collect_candidates_with(repo_dir, adapter.as_ref(), &path_suppressions, detect);
        let effective_n_cal = opts.n_cal.min(candidates.len());
        let typicality = TypicalityModel::new(language);

        // Per-corpus auto-detect: probe the rare rule's fire rate on
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
                detect.data_threshold,
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
            // Internal calibration diagnostic — noise on a normal `argot init`.
            // Only surface it when debugging (ARGOT_DEBUG set); the decision it
            // logs is already reflected in the emitted scorer-config.
            if std::env::var_os("ARGOT_DEBUG").is_some() {
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
            }
            if !keep_rule {
                resolved_rare = 0;
            }
        }

        // Leave-one-file-out counts: calibration hunks are scored as if
        // their file were not in the corpus (see multi_seed_thresholds).
        let per_file_counts: PerFileTokenCounts = corpus
            .iter()
            .map(|(p, s)| (p.clone(), bpe.token_counts(s)))
            .collect();

        let seed_thresholds = multi_seed_thresholds(
            &candidates,
            &bpe,
            Some(&per_file_counts),
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

        // Separate new-file threshold: the honest operating point for a
        // genuinely-new file (cluster routing off, real check-time alpha) —
        // never below the existing-file threshold. Applied by check only to
        // hunks whose file was absent from the fit corpus.
        let new_file_seeds = multi_seed_new_file_thresholds(
            &candidates,
            &bpe,
            Some(&per_file_counts),
            &call_receiver,
            adapter.as_ref(),
            &typicality,
            &ThresholdRunConfig {
                n_cal: effective_n_cal,
                base_seed: opts.seed,
                n_seeds: opts.n_seeds,
                cluster_bonus: CR_CLUSTER_BONUS,
                cap: CR_CAP as f64,
            },
            CR_ALPHA,
            CR_ROOT_BONUS,
        );
        let new_file_threshold = median(new_file_seeds).max(threshold);

        // Per-slice thresholds: re-calibrate over just the candidates whose file
        // falls in each slice. A slice with too few candidates is skipped — it
        // would only get a noisier threshold than the whole-repo one.
        let mut slice_configs: Vec<SliceConfig> = Vec::new();
        for slice in &resolved_slices {
            let slice_candidates: Vec<Candidate> = candidates
                .iter()
                .filter(|c| slice_matches(&rel_to_repo(&c.file_path, repo_dir), &slice.paths))
                .cloned()
                .collect();
            if slice_candidates.len() < SLICE_MIN_CANDIDATES {
                continue;
            }
            let slice_n_cal = opts.n_cal.min(slice_candidates.len());
            let slice_seeds = multi_seed_thresholds(
                &slice_candidates,
                &bpe,
                Some(&per_file_counts),
                &mut call_receiver,
                adapter.as_ref(),
                &typicality,
                &ThresholdRunConfig {
                    n_cal: slice_n_cal,
                    base_seed: opts.seed,
                    n_seeds: opts.n_seeds,
                    cluster_bonus: CR_CLUSTER_BONUS,
                    cap: CR_CAP as f64,
                },
            );
            slice_configs.push(SliceConfig {
                name: slice.name.clone(),
                paths: slice.paths.clone(),
                threshold: median(slice_seeds),
            });
        }

        // Evidence corpus.
        let evidence = build_evidence_corpus(
            &lang_files,
            adapter.as_ref(),
            &call_receiver,
            opts.evidence_top_n,
            &head,
        );

        // Convention-rarity model: corpus frequencies plus firing bars set at
        // the max feature value over the same multi-seed calibration sample
        // the threshold uses — the stage stays silent on in-voice code, and
        // per the calibration contract it never feeds the threshold itself.
        // Off by default (secondary coverage, co-headline FP driver); opt in
        // via `enable_conventions`.
        let conventions = if opts.enable_conventions {
            let mut convention_model = fit_convention_frequencies(corpus, language);
            let (syntax_bar, ident_bars) =
                calibrate_convention_bars(&candidates, &convention_model, language, &typicality);
            convention_model.syntax_bar = syntax_bar;
            convention_model.ident_bars = ident_bars;
            Some(convention_model)
        } else {
            None
        };

        // Fit-time model snapshot: the calibration call-receiver's fitted
        // state is threshold-parameter-independent (rare/alpha are scoring
        // knobs, not fitted state), so exporting from it is exact.
        let model = LanguageModel {
            bpe: bpe.stats(),
            call_receiver: call_receiver.export_model(repo_dir),
            conventions,
        };
        let model_hash = model.hash();

        // Semantic index for this language: embed every corpus function (the
        // same `callable_bodies` extraction check uses on the diff). Skipped
        // silently when no embedder is available — never load-bearing.
        #[cfg(feature = "semantic")]
        if let Some(emb) = embedder.as_ref() {
            let mut funcs = Vec::new();
            for (path, source) in corpus {
                // Mirror the check-time candidate scope (`batch.ignored_by_pattern`):
                // never index code the check would skip — tests, docs, examples,
                // scripts, migrations, generated (`argot:recommended` + `[exclude]`).
                // A reinvention target must be *canonical* repo code, not a tutorial
                // or fixture; indexing those both mis-points findings and inflates
                // self-similarity on duplication-heavy example trees.
                if path_suppressions.is_suppressed_abs(path, repo_dir) {
                    continue;
                }
                let rel = rel_to_repo(path, repo_dir);
                funcs.extend(crate::scoring::semantic::index::functions_in_file(
                    adapter.as_ref(),
                    &rel,
                    source,
                ));
            }
            if !funcs.is_empty() {
                let prior_index = prior_artifact
                    .as_ref()
                    .and_then(|a| a.load(name).ok().flatten())
                    .map(|li| li.index);
                eprintln!(
                    "argot: building semantic index for {name} ({} functions)…",
                    funcs.len()
                );
                match crate::scoring::semantic::index::SemanticIndex::build_with_reuse(
                    emb,
                    &funcs,
                    prior_index.as_ref(),
                ) {
                    Ok((idx, reused)) if !idx.is_empty() => {
                        if reused > 0 {
                            eprintln!(
                                "argot: {name}: reused {reused} of {} embeddings from the previous fit",
                                funcs.len()
                            );
                        }
                        // Self-calibrate the placement sense on the fresh index
                        // (adaptive areas, entangled merges, vote parameters —
                        // or disabled when the repo's areas aren't separable).
                        let placement =
                            crate::scoring::semantic::placement::calibrate_placement(&idx);
                        // Self-calibrate the reinvention sense: mini-replay of
                        // the functions added over the recent window against the
                        // older code (conservative mode for repos practicing
                        // systematic parallel implementation).
                        let recent = recent_semantic_functions(
                            repo_dir,
                            &funcs,
                            adapter.as_ref(),
                            SEMANTIC_CALIBRATION_WINDOW,
                        );
                        let reinvention =
                            crate::scoring::semantic::redundant::calibrate_reinvention(
                                &idx, &recent,
                            );
                        semantic_artifact.insert(name, &idx, placement, reinvention);
                    }
                    Ok(_) => {}
                    Err(e) => eprintln!("argot: semantic index for {name} failed: {e}"),
                }
            }
        }

        thresholds_out.push((name.to_string(), threshold));
        languages.insert(
            name.to_string(),
            LangConfig {
                threshold,
                new_file_threshold,
                call_receiver_alpha: CR_ALPHA,
                call_receiver_cap: CR_CAP,
                call_receiver_root_bonus: CR_ROOT_BONUS,
                call_receiver_n_clusters: CR_N_CLUSTERS,
                call_receiver_cluster_seed: CR_CLUSTER_SEED,
                call_receiver_cluster_bonus: CR_CLUSTER_BONUS,
                call_receiver_cluster_rare_threshold: resolved_rare,
                call_receiver_cluster_size_min: opts.cluster_size_min,
                call_receiver_parse_error_host_fallback: CR_PARSE_ERROR_FALLBACK,
                convention_bonus: if opts.enable_conventions {
                    CONVENTION_BONUS
                } else {
                    0.0
                },
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
                slices: slice_configs,
                model,
            },
        );
    }

    let mut corpus_files_sorted = corpus_rel.clone();
    corpus_files_sorted.sort();
    corpus_files_sorted.dedup();
    let config = ScorerConfig {
        version: CONFIG_VERSION,
        languages,
        corpus_files: corpus_files_sorted,
    };
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let json = serde_json::to_string_pretty(&config)?;
    write_atomic(output, json.as_bytes())?;

    // Semantic index artifact, alongside scorer-config.json. Kept in its own
    // file so scorer-config.json (and its model hash) stay byte-for-byte
    // unchanged whether or not the semantic layer is compiled in.
    #[cfg(feature = "semantic")]
    if !semantic_artifact.is_empty() {
        let sem_path = output.with_file_name(crate::scoring::semantic::SEMANTIC_INDEX_FILE);
        match semantic_artifact.to_json_string() {
            Ok(sem_json) => {
                if let Err(e) = write_atomic(&sem_path, sem_json.as_bytes()) {
                    eprintln!("argot: writing semantic index failed: {e}");
                }
            }
            Err(e) => eprintln!("argot: serializing semantic index failed: {e}"),
        }
    }

    // Architecture-graph artifact (`.argot/layering.json`), alongside
    // scorer-config.json. Feature-gated + in its own file so the base config is
    // byte-for-byte unchanged whether or not the layer is compiled in. Built from
    // the same voice-file collection production fits on (config-respecting) —
    // Python only in v1; other languages simply produce no graph.
    #[cfg(feature = "arch")]
    if rule_settings.severity_of_reason("layering") != crate::rules::Severity::Off {
        use crate::scoring::adapters::Language;
        use crate::scoring::arch_graph::{RepoLayering, LAYERING_FILE};
        let files = crate::train::collect_source_files(repo_dir);
        let mut sources: Vec<(String, String)> = Vec::new();
        for abs in &files {
            if abs.extension().and_then(|e| e.to_str()) != Some("py") {
                continue;
            }
            if let (Ok(rel), Ok(src)) = (abs.strip_prefix(repo_dir), std::fs::read_to_string(abs)) {
                sources.push((rel.to_string_lossy().replace('\\', "/"), src));
            }
        }
        let graph = RepoLayering::fit(
            sources.iter().map(|(p, s)| (p.as_str(), s.as_str())),
            Language::Python,
        );
        if graph.edge_count() > 0 {
            let path = output.with_file_name(LAYERING_FILE);
            if let Err(e) = write_atomic(&path, graph.to_json(&opts.repo_sha).as_bytes()) {
                eprintln!("argot: writing layering graph failed: {e}");
            }
        }
    }

    // Test-integrity gates (`.argot/integrity.json`), alongside
    // scorer-config.json — feature-gated + its own file so the base config is
    // byte-for-byte unchanged. A mini-replay over the repo's accepted-history
    // window measures each gaming event's natural rate and disables the
    // classes this repo's normal development trips too often (FP-first; see
    // the module docs of `scoring::integrity`).
    #[cfg(feature = "integrity")]
    if rule_settings.group_enabled(crate::rules::GROUP_INTEGRITY) {
        use crate::scoring::integrity::{fit_model, INTEGRITY_FILE};
        if let Some(model) = fit_model(repo_dir, &opts.repo_sha) {
            let path = output.with_file_name(INTEGRITY_FILE);
            if let Err(e) = write_atomic(&path, model.to_json().as_bytes()) {
                eprintln!("argot: writing integrity gates failed: {e}");
            }
        }
    }

    // Emit the inspectable model manifest alongside the config.
    let per_lang_model_hash: BTreeMap<String, String> = config
        .languages
        .iter()
        .map(|(lang, lc)| (lang.clone(), lc.model_hash.clone()))
        .collect();
    let lang_summaries: Vec<LangSummary> = config
        .languages
        .iter()
        .map(|(lang, lc)| LangSummary {
            language: lang.clone(),
            threshold: lc.threshold,
            model_hash: lc.model_hash.clone(),
            n_cal: lc.calibration.n_cal,
            files: per_lang_files.get(lang).copied().unwrap_or(0),
        })
        .collect();
    let manifest = Manifest {
        manifest_version: MANIFEST_VERSION,
        config_version: CONFIG_VERSION,
        model_hash: combined_model_hash(&per_lang_model_hash),
        scorer_config_hash: short_hash(json.as_bytes()),
        fit_commit_sha: opts.repo_sha.clone(),
        fit_timestamp: opts.timestamp_utc.clone(),
        corpus: CorpusSummary {
            files: corpus_files.len(),
            lines: total_lines,
        },
        languages: lang_summaries,
    };
    if let Some(parent) = output.parent() {
        let manifest_path = parent.join(MANIFEST_FILE);
        if let Ok(manifest_json) = serde_json::to_string_pretty(&manifest) {
            write_atomic(&manifest_path, manifest_json.as_bytes())?;
        }
        // Fit-time self-diagnosis, persisted so `check`/`status` surface it
        // without re-scanning (and so a background refit's findings survive
        // its /dev/null stdout): calibration-drift candidates + the config
        // fingerprint this fit reflects.
        let drift_candidates = crate::suppress::suggest_ignores(repo_dir)
            .candidates
            .into_iter()
            .map(|c| c.path)
            .collect();
        crate::health::write(
            parent,
            &crate::health::FitHealth {
                fit_sha: opts.repo_sha.clone(),
                config_fingerprint: config_fingerprint_at_fit,
                drift_candidates,
            },
        );
    }

    Ok(thresholds_out)
}

/// Build the evidence corpus (`build_evidence_corpus`).
fn build_evidence_corpus(
    files: &[PathBuf],
    adapter: &dyn LanguageAdapter,
    call_receiver: &CallReceiverScorer,
    top_n: usize,
    head: &HeadSource,
) -> EvidenceCorpusJson {
    use std::collections::HashMap;
    // imports: per-file distinct specifiers.
    let mut import_counts: HashMap<String, usize> = HashMap::new();
    let mut identifier_counts: HashMap<String, usize> = HashMap::new();
    let noise = adapter.identifier_noise();
    for path in files {
        let source = match head.read(path) {
            Some(s) => s,
            None => continue,
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

/// History window (first-parent commits) for the semantic self-calibrations —
/// mirrors the clean-commit bench window.
#[cfg(feature = "semantic")]
const SEMANTIC_CALIBRATION_WINDOW: usize = 150;

/// Which of `funcs` (index order) were ADDED within the recent history window:
/// their file is new since the window commit, or the file changed and the old
/// version (parsed with the same adapter — names only, no embedding) did not
/// define their symbol. Empty history / any git failure → all-false (the
/// reinvention calibration then keeps the standard rule).
#[cfg(feature = "semantic")]
fn recent_semantic_functions(
    repo_dir: &Path,
    funcs: &[crate::scoring::semantic::index::FunctionRef],
    adapter: &dyn LanguageAdapter,
    window: usize,
) -> Vec<bool> {
    let none = vec![false; funcs.len()];
    let Ok(repo) = git2::Repository::open(repo_dir) else {
        return none;
    };
    let Ok(head) = repo.head().and_then(|h| h.peel_to_commit()) else {
        return none;
    };
    // Walk `window` first-parent steps back (the bench's HEAD~window).
    let mut old = head.clone();
    for _ in 0..window {
        match old.parent(0) {
            Ok(p) => old = p,
            Err(_) => return none, // history shorter than the window
        }
    }
    let (Ok(old_tree), Ok(head_tree)) = (old.tree(), head.tree()) else {
        return none;
    };
    let Ok(diff) = repo.diff_tree_to_tree(Some(&old_tree), Some(&head_tree), None) else {
        return none;
    };
    let mut changed: std::collections::HashSet<String> = std::collections::HashSet::new();
    diff.foreach(
        &mut |delta, _| {
            if let Some(p) = delta.new_file().path().and_then(|p| p.to_str()) {
                changed.insert(p.replace('\\', "/"));
            }
            true
        },
        None,
        None,
        None,
    )
    .ok();
    // Old symbol sets for changed files that already existed at the old commit.
    let mut old_syms: std::collections::HashMap<String, std::collections::HashSet<String>> =
        std::collections::HashMap::new();
    for path in funcs.iter().map(|f| f.path.as_str()) {
        if !changed.contains(path) || old_syms.contains_key(path) {
            continue;
        }
        let syms = old_tree
            .get_path(Path::new(path))
            .ok()
            .and_then(|e| e.to_object(&repo).ok())
            .and_then(|o| o.into_blob().ok())
            .map(|b| String::from_utf8_lossy(b.content()).into_owned())
            .map(|src| {
                crate::scoring::semantic::index::functions_in_file(adapter, path, &src)
                    .into_iter()
                    .map(|f| f.symbol)
                    .collect::<std::collections::HashSet<String>>()
            })
            .unwrap_or_default(); // absent at the old commit → new file → empty set
        old_syms.insert(path.to_string(), syms);
    }
    funcs
        .iter()
        .map(|f| {
            changed.contains(&f.path)
                && !old_syms
                    .get(&f.path)
                    .map(|s| s.contains(&f.symbol))
                    .unwrap_or(false)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_for_filename_ctx_resolves_dot_h_by_repo_majority() {
        // `.h` follows the repo decision; nothing else moves.
        assert_eq!(language_for_filename_ctx("x.h", true), Some(Language::Cpp));
        assert_eq!(language_for_filename_ctx("x.h", false), Some(Language::C));
        assert_eq!(language_for_filename_ctx("x.c", true), Some(Language::C));
        assert_eq!(
            language_for_filename_ctx("x.cpp", false),
            Some(Language::Cpp)
        );
        assert_eq!(
            language_for_filename_ctx("x.py", true),
            Some(Language::Python)
        );
    }

    #[test]
    fn header_is_cpp_follows_translation_unit_majority() {
        let base = std::env::temp_dir().join(format!("argot_hdr_{}", std::process::id()));

        // C++-majority: more .cc than .c → headers are C++.
        let cpp_repo = base.join("cpp");
        std::fs::create_dir_all(&cpp_repo).unwrap();
        for f in ["a.cc", "b.cc", "core.h", "util.c"] {
            std::fs::write(cpp_repo.join(f), "x\n").unwrap();
        }
        assert!(header_is_cpp(&cpp_repo), "2 .cc vs 1 .c → C++");

        // C-majority: more .c than C++ TUs → headers are C.
        let c_repo = base.join("c");
        std::fs::create_dir_all(&c_repo).unwrap();
        for f in ["a.c", "b.c", "c.c", "net.h"] {
            std::fs::write(c_repo.join(f), "x\n").unwrap();
        }
        assert!(!header_is_cpp(&c_repo), "3 .c vs 0 C++ TU → C");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn head_source_reads_committed_version_ignoring_working_tree_edits() {
        use std::process::Command;
        let dir = std::env::temp_dir().join(format!("argot_headsrc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(&dir)
                .output()
                .expect("git");
        };
        git(&["init", "-q"]);
        git(&["config", "user.email", "t@t.co"]);
        git(&["config", "user.name", "t"]);
        std::fs::write(dir.join("a.py"), "import json\n").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-qm", "init"]);
        // Working-tree edit to a tracked file + a brand-new untracked file.
        std::fs::write(dir.join("a.py"), "import json\nimport requests\n").unwrap();
        std::fs::write(dir.join("b.py"), "import os\n").unwrap();

        let head = HeadSource::new(&dir);
        // Tracked file → its committed HEAD version, NOT the uncommitted edit.
        assert_eq!(
            head.read(&dir.join("a.py")).as_deref(),
            Some("import json\n"),
            "a modified tracked file must fit from HEAD, not the working tree"
        );
        // Untracked file → read as-is from disk (nested/non-repo callers unchanged).
        assert_eq!(head.read(&dir.join("b.py")).as_deref(), Some("import os\n"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn slice_matches_by_prefix_and_exact_file() {
        let paths = vec!["frontend/".to_string(), "shared/util.ts".to_string()];
        assert!(slice_matches("frontend/app.ts", &paths));
        assert!(slice_matches("shared/util.ts", &paths)); // exact file
        assert!(!slice_matches("backend/api.py", &paths));
        assert!(!slice_matches("shared/other.ts", &paths));
    }

    #[test]
    fn auto_slices_are_top_level_dirs_above_the_floor() {
        let mut files: Vec<String> = Vec::new();
        // frontend/ has enough files; docs/ does not.
        for i in 0..SLICE_AUTO_MIN_FILES {
            files.push(format!("frontend/f{i}.ts"));
        }
        files.push("docs/readme.ts".to_string());
        files.push("top.ts".to_string()); // no dir → ignored
        let slices = auto_slices(&files);
        let names: Vec<&str> = slices.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"path:frontend/"));
        assert!(!names.iter().any(|n| n.contains("docs")));
    }

    #[test]
    fn resolve_slices_parses_path_specs_and_ignores_unknown() {
        let slices = resolve_slices(
            Path::new("/nonexistent"),
            &[],
            &[
                "path:frontend/".to_string(),
                "bogus".to_string(),
                "path:".to_string(), // empty → dropped
            ],
        );
        assert_eq!(slices.len(), 1);
        assert_eq!(slices[0].name, "path:frontend/");
        assert_eq!(slices[0].paths, vec!["frontend/".to_string()]);
    }
}
