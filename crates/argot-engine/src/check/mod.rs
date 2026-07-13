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
//! This module is the rule-agnostic half of `check`: patch collection
//! ([`collect`]), rendering ([`render`]), and the orchestration loop
//! ([`orchestrate`] — [`run_check`] and the freshness/review-mutes plumbing).
//! [`run_check`] takes the run's registered [`crate::detector::Detector`]s as
//! a parameter — which rule groups actually run (the base voice pass, plus
//! whichever of semantic/architecture/integrity a build compiles in) is
//! decided by the composition root one layer up (`argot-core`'s
//! `compose::default_detectors`), never by this crate.

mod collect;
mod orchestrate;
pub mod render;
pub mod two_sided;

#[cfg(test)]
mod tests;

pub use orchestrate::{
    accepted_anchor, accepted_source_commits_behind, commits_since_fit, freshness_anchor,
    in_scope_commits_between, run_check, run_review_mutes, unmerged_branch_source_commits,
    ReviewOutcome, FRESHNESS_SCAN_CAP,
};

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
///
/// Fields are `pub` (not `pub(crate)`): the feature-gated rule-group passes
/// that read patch content (`voice`/`semantic`/`arch`/`integrity`) live in
/// `argot-core`'s `check_passes`, a different crate, since this is the
/// rule-agnostic engine they plug into.
pub struct PatchBatch {
    pub file_path: String,
    pub content: Vec<u8>,
    pub hunks: Vec<HunkSpan>,
    pub source: String,
    /// The file matched a user `[exclude].paths` pattern: still scored (so the
    /// suppression is countable), but every hit is dropped from output and
    /// exit-code consideration.
    pub ignored_by_pattern: bool,
}

// Extension → language routing — a thin re-export of `argot-lang`'s `ext`
// module (moved there since it's language-substrate, not check-specific
// logic, and both `check` and out-of-process consumers depend on it).
pub use argot_lang::ext::{ext_to_lang, ext_to_lang_ctx, extension};
