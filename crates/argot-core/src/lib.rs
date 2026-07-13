//! argot-core — the voice-linter engine.
//!
//! This crate is a behaviour-preserving port of the Python engine
//! (`engine/argot`). It is deliberately language- and corpus-agnostic: no
//! hardcoded framework/language literals leak into scoring logic (per the
//! project's `CLAUDE.md`). Framework-specific knowledge lives only in
//! fixtures, benchmarks, and eval code — never here.
//!
//! Pipeline: extract → train → calibrate → check.
//!
//! The rule-agnostic half of the engine (the [`crate::detector::Detector`]
//! contract, `check` orchestration, config, suppression surfaces, git/corpus
//! walking, and the shared rendering/registry plumbing) has moved to the
//! `argot-engine` crate. This crate re-exports every engine module at its
//! historical path (below) so existing callers keep compiling unchanged, and
//! adds [`compose`] — the composition root deciding which rule groups
//! (`check_passes`: voice always, semantic/architecture/integrity per cargo
//! feature) this build wires into the engine's `check` loop.

pub mod check_passes;
pub(crate) mod compose;
pub mod extract;
mod ignore_suggest;
pub mod inspect;
pub mod scoring;
pub mod train;

/// The rule-agnostic engine, re-exported at its historical paths so every
/// existing `argot_core::{config,rules,output,git_walk,health,
/// timing,json,stats,cache,par,finding,detector,corpus}` (and internal
/// `crate::...`) caller keeps resolving unchanged.
pub use argot_engine::{
    cache, config, corpus, detector, finding, git_walk, health, json, output, par, rules, stats,
    timing,
};

/// `check` is a thin facade: everything rule-agnostic re-exports straight from
/// `argot_engine::check`; `run_check` and `RepoScorers` are argot-core's own
/// (the former closes over this build's composition root, the latter lives in
/// `check_passes::load` alongside the other feature-gated rule passes).
pub mod check {
    pub use crate::check_passes::load::RepoScorers;
    pub use argot_engine::check::{
        accepted_anchor, accepted_source_commits_behind, commits_since_fit, ext_to_lang,
        ext_to_lang_ctx, extension, freshness_anchor, in_scope_commits_between, run_review_mutes,
        unmerged_branch_source_commits, CheckArgs, CheckOutcome, ReviewOutcome, DEFAULT_HUNK_LINES,
        FRESHNESS_SCAN_CAP,
    };

    /// Entry point: wires this build's registered detectors (the composition
    /// root, `crate::compose::default_detectors`) into the engine's
    /// rule-agnostic `run_check` loop.
    pub fn run_check(args: CheckArgs) -> CheckOutcome {
        argot_engine::check::run_check(args, crate::compose::default_detectors())
    }
}

/// `suppress` is engine's module plus `ignore_suggest`, which stays in this
/// crate: it needs `inspect::adapter_for` and
/// `scoring::calibration::language_for_filename`, both downstream of the
/// rule-agnostic engine, so it cannot live on that side without an illegal
/// engine → core dependency.
pub mod suppress {
    pub use crate::ignore_suggest::{suggest_ignores, IgnoreCandidate, IgnoreSuggestions};
    pub use argot_engine::suppress::*;
}

// Language-substrate modules (adapters, tokenization, BPE, the dataset wire
// format) live in the leaf crate `argot-lang` and are re-exported here at
// their original paths so every existing `argot_core::bpe`/`dataset`/`text`/
// `tokenize` (and, via `scoring::mod`, `scoring::adapters`/`scoring::filters`)
// caller keeps compiling unchanged.
pub use argot_lang::{bpe, dataset, text, tokenize};
