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
//!
//! Submodules, by responsibility: [`load`] (per-language scorer loading),
//! [`collect`] (git patch collection), [`voice`] (the base statistical pass),
//! [`render`] (human/machine rendering), [`orchestrate`] (`run_check` and the
//! freshness/review-mutes plumbing), plus the feature-gated additive passes
//! [`semantic`], [`arch`], [`integrity`].

mod collect;
mod load;
mod orchestrate;
mod render;
mod voice;

#[cfg(feature = "arch")]
mod arch;
#[cfg(feature = "integrity")]
mod integrity;
#[cfg(feature = "semantic")]
mod semantic;

#[cfg(test)]
mod tests;

pub use load::RepoScorers;
pub use orchestrate::{
    accepted_anchor, accepted_source_commits_behind, commits_since_fit, freshness_anchor,
    in_scope_commits_between, run_check, run_review_mutes, unmerged_branch_source_commits,
    ReviewOutcome, FRESHNESS_SCAN_CAP,
};

#[cfg(feature = "arch")]
pub(crate) use arch::ArchDetector;
#[cfg(feature = "integrity")]
pub(crate) use integrity::IntegrityDetector;

use crate::git_walk::HunkSpan;
use crate::output::OutputFormat;
use crate::rules::RulesLayer;
use std::path::PathBuf;

/// Default number of hunk-body lines shown under each above-threshold hit.
pub const DEFAULT_HUNK_LINES: usize = 6;

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
