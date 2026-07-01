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
//! when the config carries an `evidence_corpus` block. Syntax highlighting and
//! the ANSI color path remain deferred.

use crate::git_walk::{
    open_repo, resolve_shas, walk_commits, HunkSpan, WalkItem, SUPPORTED_EXTENSIONS,
};
use crate::output::{render_json, render_sarif, HitRecord, OutputFormat, ReportMeta};
use crate::scoring::adapters::python::PythonAdapter;
use crate::scoring::adapters::typescript::TypeScriptAdapter;
use crate::scoring::adapters::LanguageAdapter;
use crate::scoring::evidence::types::{Evidence, EvidenceCorpus, SourceSpan};
use crate::scoring::evidence::{evidence_caret_spans, evidence_lines_of_interest, format_evidence};
use crate::scoring::sequential::{SequentialConfig, SequentialImportBpeScorer};
use crate::text::{read_text_lossy, splitlines};
use git2::{DiffFindOptions, Patch, Status, StatusOptions};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

/// Default number of hunk-body lines shown under each above-threshold hit.
pub const DEFAULT_HUNK_LINES: usize = 6;

/// Severity tier ordering, weakest first (`_SEVERITY_ORDER`).
const SEVERITY_ORDER: [&str; 3] = ["unusual", "suspicious", "foreign"];

/// Directory / filename exclusions mirrored from the calibration corpus
/// (`random_hunk_sampler.DEFAULT_EXCLUDE_DIRS`).
const DEFAULT_EXCLUDE_DIRS: &[&str] = &[
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
    pub min_severity: String,
    pub use_color: bool,
    /// Output format. Machine formats (`json`/`sarif`) own stdout exclusively.
    pub format: OutputFormat,
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
}

/// One above-threshold hunk plus everything needed to explain it (`_Hit`).
struct Hit {
    /// BPE-stage score (`scored.stages.bpe_score`), regardless of winning reason.
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
}

/// Loaded per-language scorers plus the filtering machinery.
struct Loaded {
    scorers: HashMap<String, SequentialImportBpeScorer>,
    filter_adapters: HashMap<String, Box<dyn LanguageAdapter>>,
    language_extensions: HashSet<String>,
}

/// Extension → language name (`_EXT_TO_LANG`). JS/JSX route to TypeScript.
const EXT_TO_LANG: &[(&str, &str)] = &[
    (".py", "python"),
    (".ts", "typescript"),
    (".tsx", "typescript"),
    (".js", "typescript"),
    (".jsx", "typescript"),
];

fn ext_to_lang(ext: &str) -> Option<&'static str> {
    EXT_TO_LANG.iter().find(|(e, _)| *e == ext).map(|(_, l)| *l)
}

fn adapter_for_language(lang: &str) -> Option<Box<dyn LanguageAdapter>> {
    match lang {
        "python" => Some(Box::new(PythonAdapter::new())),
        "typescript" => Some(Box::new(TypeScriptAdapter::new())),
        _ => None,
    }
}

/// Python `Path(path).suffix.lower()` (`git_walk._extension`).
fn extension(path: &str) -> String {
    let name = match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    };
    match name.rfind('.') {
        Some(i) if i > 0 && i < name.len() - 1 => name[i..].to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// Case-sensitive `language_for_extension(Path(p).suffix)` used for corpus
/// partitioning (NOT lowercased, matching the Python calibration path).
fn lang_for_ext_cased(p: &Path) -> Option<&'static str> {
    let name = p.file_name()?.to_str()?;
    let suffix = match name.rfind('.') {
        Some(i) if i > 0 && i < name.len() - 1 => &name[i..],
        _ => "",
    };
    match suffix {
        ".py" => Some("python"),
        ".ts" | ".tsx" | ".js" | ".jsx" => Some("typescript"),
        _ => None,
    }
}

fn is_supported_ext(file_path: &str) -> bool {
    SUPPORTED_EXTENSIONS.contains(&extension(file_path).as_str())
}

fn sev_index(s: &str) -> usize {
    SEVERITY_ORDER.iter().position(|x| *x == s).unwrap_or(0)
}

/// Classify a score into a severity tier relative to a calibrated threshold
/// (`_severity`).
fn severity(score: f64, threshold: f64) -> &'static str {
    if score >= threshold + 1.5 {
        "foreign"
    } else if score >= threshold + 0.5 {
        "suspicious"
    } else {
        "unusual"
    }
}

/// User-facing translation of a scorer `reason` code (`_REASON_LABEL`).
fn reason_label(reason: &str) -> &str {
    match reason {
        "bpe" => "rare token sequence",
        "call_receiver" => "unfamiliar callee",
        "import" => "foreign import",
        other => other,
    }
}

/// Port of `is_excluded_path`. `file_path` is already relative to the repo root
/// (git paths are `/`-separated), so the `relative_to` step is a no-op here.
fn is_excluded_path(file_path: &str) -> bool {
    let parts: Vec<&str> = file_path.split('/').collect();
    if parts.len() >= 2 {
        for part in &parts[..parts.len() - 1] {
            if DEFAULT_EXCLUDE_DIRS.contains(part)
                || part.starts_with("test")
                || *part == "__tests__"
            {
                return true;
            }
        }
    }
    let name = *parts.last().unwrap_or(&file_path);
    if name.starts_with("test_") || name == "conftest.py" {
        return true;
    }
    if name.contains(".test.") || name.contains(".spec.") {
        return true;
    }
    if name.contains(".config.") {
        return true;
    }
    name.starts_with('.') && name.get(1..).map(|r| r.contains("rc.")).unwrap_or(false)
}

/// Port of `_is_out_of_scope`: wrong language, excluded path, or data-dominant.
fn is_out_of_scope(
    file_path: &str,
    content: &[u8],
    language_extensions: &HashSet<String>,
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
) -> bool {
    let ext = extension(file_path);
    if !language_extensions.contains(&ext) {
        return true;
    }
    if is_excluded_path(file_path) {
        return true;
    }
    let source = String::from_utf8_lossy(content);
    match ext_to_lang(&ext).and_then(|l| filter_adapters.get(l)) {
        Some(adapter) => adapter.is_data_dominant(&source),
        None => true,
    }
}

/// Shell-style glob match (`fnmatch.fnmatch`), case-sensitive (posix normcase).
fn fnmatch(name: &str, pat: &str) -> bool {
    let n: Vec<char> = name.chars().collect();
    let p: Vec<char> = pat.chars().collect();
    fn rec(n: &[char], p: &[char]) -> bool {
        if p.is_empty() {
            return n.is_empty();
        }
        match p[0] {
            '*' => rec(n, &p[1..]) || (!n.is_empty() && rec(&n[1..], p)),
            '?' => !n.is_empty() && rec(&n[1..], &p[1..]),
            '[' => {
                if n.is_empty() {
                    return false;
                }
                let mut i = 1;
                let neg = i < p.len() && p[i] == '!';
                if neg {
                    i += 1;
                }
                let mut set: Vec<char> = Vec::new();
                let mut ranges: Vec<(char, char)> = Vec::new();
                let mut first = true;
                let mut closed = false;
                let mut j = i;
                while j < p.len() {
                    if p[j] == ']' && !first {
                        closed = true;
                        j += 1;
                        break;
                    }
                    if j + 2 < p.len() && p[j + 1] == '-' && p[j + 2] != ']' {
                        ranges.push((p[j], p[j + 2]));
                        j += 3;
                    } else {
                        set.push(p[j]);
                        j += 1;
                    }
                    first = false;
                }
                if !closed {
                    return n[0] == '[' && rec(&n[1..], &p[1..]);
                }
                let c = n[0];
                let mut matched =
                    set.contains(&c) || ranges.iter().any(|(a, b)| *a <= c && c <= *b);
                if neg {
                    matched = !matched;
                }
                matched && rec(&n[1..], &p[j..])
            }
            ch => !n.is_empty() && n[0] == ch && rec(&n[1..], &p[1..]),
        }
    }
    rec(&n, &p)
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

/// Load v2 per-language scorers from `.argot/` (`_load_scorers` + helpers).
/// On failure returns the exact stderr message and exit code.
fn load_scorers(argot_dir: &Path) -> Result<Loaded, (String, i32)> {
    let repo_corpus_txt = argot_dir.join("repo-corpus.txt");
    let generic_baseline_json = argot_dir.join("generic-baseline.json");
    let config_json = argot_dir.join("scorer-config.json");

    for (p, msg) in [
        (&repo_corpus_txt, "run `argot fit` first"),
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

    if config.get("version").and_then(Value::as_i64) != Some(2) {
        let vrepr = config
            .get("version")
            .map(py_repr)
            .unwrap_or_else(|| "None".to_string());
        return Err((
            format!(
                "error: {} uses config version {} — regenerate via `argot-calibrate`.\n",
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

    let corpus_text =
        fs::read_to_string(&repo_corpus_txt).map_err(|e| (format!("error: {e}\n"), 2))?;
    let repo_corpus_files: Vec<PathBuf> = splitlines(&corpus_text)
        .into_iter()
        .filter(|l| !l.trim().is_empty())
        .map(PathBuf::from)
        .collect();

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
            import_modules: get_strings("import_modules"),
            import_module_prefixes: get_strings("import_module_prefixes"),
            // Parse the optional `evidence_corpus` block. Unlike the Python
            // loader (which requires it), the Rust port keeps evidence optional:
            // a config without the block simply renders no `↳` evidence lines,
            // so the pre-evidence check goldens stay byte-identical.
            evidence_corpus: lc
                .get("evidence_corpus")
                .and_then(EvidenceCorpus::from_json),
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

        let lang_files: Vec<PathBuf> = repo_corpus_files
            .iter()
            .filter(|p| lang_for_ext_cased(p) == Some(lang.as_str()))
            .cloned()
            .collect();
        let repo_files: Vec<(PathBuf, String)> = lang_files
            .iter()
            .filter_map(|p| read_text_lossy(p).ok().map(|s| (p.clone(), s)))
            .collect();

        let scorer =
            SequentialImportBpeScorer::from_config(&repo_files, &baseline_bytes, adapter, cfg)
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

    Ok(Loaded {
        scorers,
        filter_adapters,
        language_extensions,
    })
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
        });
        Ok(ControlFlow::Continue(()))
    })?;
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

/// Score each hunk, dispatching per language (`_score_patches`). Returns
/// `(hits, hunk_count)`.
fn score_patches(
    patches: Vec<PatchBatch>,
    scorers: &mut HashMap<String, SequentialImportBpeScorer>,
    stderr: &mut String,
) -> (Vec<Hit>, usize) {
    let mut hits: Vec<Hit> = Vec::new();
    let mut hunk_count = 0usize;

    for batch in patches {
        let ext = extension(&batch.file_path);
        let scorer = match ext_to_lang(&ext).and_then(|l| scorers.get_mut(l)) {
            Some(s) => s,
            None => {
                stderr.push_str(&format!(
                    "[argot] skipping {}: no scorer for extension '{}'\n",
                    batch.file_path, ext
                ));
                continue;
            }
        };
        let bpe_threshold = scorer.bpe_threshold;

        let file_source = String::from_utf8_lossy(&batch.content).into_owned();
        let file_lines = splitlines(&file_source);
        let n_lines = file_lines.len() as i64;

        for hunk in &batch.hunks {
            hunk_count += 1;
            let hunk_start = hunk.new_start as i64 - 1;
            let hunk_end = hunk_start + hunk.new_lines as i64;
            if hunk_start < 0 || hunk_end > n_lines {
                continue;
            }
            let hs = hunk_start as usize;
            let he = hunk_end as usize;
            let hunk_content = file_lines[hs..he].join("\n");
            let scored = scorer.score_hunk(
                &hunk_content,
                Some(&file_source),
                Some(hs + 1),
                Some(he),
                None,
            );
            hits.push(Hit {
                score: scored.stages.bpe_score,
                file_path: batch.file_path.clone(),
                line: hunk.new_start as usize,
                line_end: (hunk.new_start + hunk.new_lines).saturating_sub(1) as usize,
                source: batch.source.clone(),
                reason: scored.reason.as_str().to_string(),
                flagged: scored.flagged,
                threshold: bpe_threshold,
                hunk_content,
                evidence: scored.evidence,
            });
        }
    }

    (hits, hunk_count)
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
                out.push(caret);
            }
        }
    }
    if overflow > 0 {
        let plural = if overflow != 1 { "s" } else { "" };
        out.push(format!(
            "  {}   (+{} more line{})",
            " ".repeat(width),
            overflow,
            plural
        ));
    }
    (out, overflow)
}

/// Render grouped results (`_render_results`, `use_color=false`). Returns
/// whether any hunk body was truncated.
fn render_results(hits: &[&Hit], hunk_lines: Option<usize>, out: &mut String) -> bool {
    // Banner tier counts use the per-hit calibrated threshold.
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for h in hits {
        *counts.entry(severity(h.score, h.threshold)).or_insert(0) += 1;
    }
    let total = hits.len();
    let mut tier_parts: Vec<String> = Vec::new();
    for tier in ["foreign", "suspicious", "unusual"] {
        let c = *counts.get(tier).unwrap_or(&0);
        if c > 0 {
            tier_parts.push(format!("{c} {tier}"));
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
        out.push_str(fp);
        out.push('\n');

        let mut fhits: Vec<&Hit> = file_hits[fp].clone();
        fhits.sort_by_key(|h| h.line); // stable by line asc

        for h in &fhits {
            let sev = severity(h.score, h.threshold);
            let line_str = if h.line == h.line_end {
                format!("L{}", h.line)
            } else {
                format!("L{}-L{}", h.line, h.line_end)
            };
            let friendly = reason_label(&h.reason);
            let reason_str = if friendly != h.reason {
                format!("{} ({})", friendly, h.reason)
            } else {
                h.reason.clone()
            };
            let meta = format!("· {} · {}", h.source, reason_str);
            let glyph = match sev {
                "foreign" => "!",
                "suspicious" => "?",
                _ => ".",
            };
            out.push_str(&format!(
                "  {}  {:<13} {:>6.2}  {}  {}\n",
                glyph, line_str, h.score, sev, meta
            ));

            // Per-reason evidence (names + `common here:`) sits between the
            // headline and the hunk body. `hunk_start_line = h.line` lets import
            // evidence render `(L7)` file-line annotations.
            if let Some(ev) = &h.evidence {
                for line in format_evidence(ev, false, h.line) {
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

/// Flatten visible hits into serializable [`HitRecord`]s for the machine
/// formats. Severity is measured against the per-hit calibrated threshold,
/// matching the human rendering; evidence lines are the same per-reason lines
/// the human path prints, with layout indentation stripped.
fn hit_records(hits: &[&Hit]) -> Vec<HitRecord> {
    hits.iter()
        .map(|h| HitRecord {
            path: h.file_path.clone(),
            line_start: h.line,
            line_end: h.line_end,
            score: h.score,
            threshold: h.threshold,
            severity: severity(h.score, h.threshold).to_string(),
            reason: h.reason.clone(),
            reason_label: reason_label(&h.reason).to_string(),
            source: h.source.clone(),
            evidence: h
                .evidence
                .as_ref()
                .map(|ev| {
                    format_evidence(ev, false, h.line)
                        .into_iter()
                        .map(|l| l.trim().to_string())
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect()
}

fn report_meta(args: &CheckArgs, scanned: String, hunks_scanned: usize) -> ReportMeta {
    ReportMeta {
        // The workspace shares one version across crates, so this matches the
        // CLI binary's version.
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        repo: args.repo_path.clone(),
        scanned,
        hunks_scanned,
    }
}

/// Render the complete machine-format document (json/sarif) for stdout.
fn render_machine(format: OutputFormat, meta: &ReportMeta, records: &[HitRecord]) -> String {
    match format {
        OutputFormat::Sarif => render_sarif(meta, records),
        _ => render_json(meta, records),
    }
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
        if reference.contains("..") {
            let shas = resolve_shas(&repo, reference)
                .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
            if shas.is_empty() {
                // Note: exit 0 (not 2) for an empty explicit range.
                return Err(CheckOutcome::err(
                    format!("No commits found in range '{reference}'\n"),
                    0,
                ));
            }
            let patches = committed_patches(repo_path, &shas)
                .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
            return Ok((patches, format!("{} commit(s) ({reference})", shas.len())));
        }
        // Bare ref: <ref>..HEAD commits plus full workdir.
        let shas = resolve_shas(&repo, &format!("{reference}..HEAD"))
            .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
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

    let Loaded {
        mut scorers,
        filter_adapters,
        language_extensions,
    } = match load_scorers(&args.argot_dir) {
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
                let meta = report_meta(&args, format!("0 commit(s) ({})", args.reference), 0);
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

    // Scope + only/exclude filters.
    let filtered: Vec<PatchBatch> = patches
        .into_iter()
        .filter(|b| {
            !is_out_of_scope(
                &b.file_path,
                &b.content,
                &language_extensions,
                &filter_adapters,
            ) && passes_filters(&b.file_path, &args.only, &args.exclude)
        })
        .collect();

    let (hits, hunk_count) = score_patches(filtered, &mut scorers, &mut stderr);

    // Display gate: --threshold widens to every hit >= N; otherwise show flagged.
    let threshold_override = args.threshold;
    let above: Vec<&Hit> = if let Some(t) = threshold_override {
        hits.iter().filter(|h| h.score >= t).collect()
    } else {
        hits.iter().filter(|h| h.flagged).collect()
    };

    // --min-severity drops weaker tiers from both output and banner counts.
    let min_idx = sev_index(&args.min_severity);
    let visible: Vec<&Hit> = above
        .iter()
        .copied()
        .filter(|h| {
            let t = threshold_override.unwrap_or(h.threshold);
            sev_index(severity(h.score, t)) >= min_idx
        })
        .collect();

    // Machine formats: the serialized document is the entire stdout; skip
    // warnings stay on stderr. Exit semantics match the human path (1 when
    // any hit is visible, 0 otherwise).
    if args.format.is_machine() {
        let records = hit_records(&visible);
        let meta = report_meta(&args, scan_label, hunk_count);
        let exit_code = if visible.is_empty() { 0 } else { 1 };
        return CheckOutcome {
            stdout: render_machine(args.format, &meta, &records),
            stderr,
            exit_code,
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
                "All {} hit(s) below severity '{}' — pass a lower --min-severity to see them.\n",
                above.len(),
                args.min_severity
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
    let any_truncated = render_results(&visible, hunk_lines, &mut stdout);

    if any_truncated && !args.verbose {
        stdout.push('\n');
        stdout.push_str("tip: pass --verbose (-v) to expand truncated hunks.\n");
    }

    CheckOutcome {
        stdout,
        stderr,
        exit_code: 1,
    }
}
