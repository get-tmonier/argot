//! argot-core — facade + composition root.
//!
//! This crate is deliberately thin: it holds no scoring or rule logic of its
//! own. Every rule group lives in its own crate — `argot-rules-voice` (the
//! base statistical guardrail: git walk, tokenization, BPE surprise,
//! call-receiver, conventions, typicality, calibration; always wired in, not
//! a cargo feature) plus the additive `argot-rules-{semantic,arch,integrity}`
//! slices (each its own crate, pulled in as an optional dependency per cargo
//! feature) — and the rule-agnostic engine lives in `argot-engine` (the
//! [`crate::detector::Detector`] contract, `check` orchestration, config,
//! suppression surfaces, git/corpus walking, shared rendering/registry
//! plumbing).
//!
//! This crate does two jobs: [`compose`], the composition root deciding
//! which rule groups this build wires into the engine's `check` and
//! `calibrate` loops; and re-exporting every module at its historical
//! `argot_core::...` path so existing callers (the CLI, the bench harness,
//! this crate's own `tests/` integration suite) keep compiling unchanged.

pub(crate) mod compose;

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
/// `argot-rules-voice`, the crate that owns the base model's load path).
pub mod check {
    pub use argot_engine::check::{
        accepted_anchor, accepted_source_commits_behind, commits_since_fit, ext_to_lang,
        ext_to_lang_ctx, extension, freshness_anchor, in_scope_commits_between, run_review_mutes,
        unmerged_branch_source_commits, CheckArgs, CheckOutcome, ReviewOutcome, DEFAULT_HUNK_LINES,
        FRESHNESS_SCAN_CAP,
    };
    pub use argot_rules_voice::load::RepoScorers;

    /// Entry point: wires this build's registered detectors (the composition
    /// root, `crate::compose::default_detectors`) into the engine's
    /// rule-agnostic `run_check` loop.
    pub fn run_check(args: CheckArgs) -> CheckOutcome {
        argot_engine::check::run_check(args, crate::compose::default_detectors())
    }
}

/// `suppress` is engine's module plus `ignore_suggest`, which lives in
/// `argot-rules-voice`: it needs `inspect::adapter_for` and
/// `scoring::calibration::language_for_filename`, both downstream of the
/// rule-agnostic engine, so it cannot live on that side without an illegal
/// engine → voice dependency.
pub mod suppress {
    pub use argot_engine::suppress::*;
    pub use argot_rules_voice::ignore_suggest::{
        suggest_ignores, IgnoreCandidate, IgnoreSuggestions,
    };
}

/// `extract`/`inspect`/`train` are argot-rules-voice's own pipeline stages,
/// re-exported here at their historical `argot_core::{extract,inspect,train}`
/// paths.
pub use argot_rules_voice::{extract, inspect, train};

/// Scoring engine facade: the voice slice's own scoring modules re-exported at
/// their historical `argot_core::scoring::*` paths, plus the language
/// substrate (`adapters`/`filters`) and the additive semantic/architecture/
/// integrity senses (each its own crate, present only when this build was
/// compiled with the matching cargo feature).
pub mod scoring {
    pub use argot_rules_voice::scoring::{
        bpe_scorer, call_receiver, conventions, evidence, import_graph, model, sequential,
        shape_primitive, shape_primitives, typicality,
    };

    /// `calibration` is almost entirely a re-export of
    /// `argot_rules_voice::scoring::calibration` — except `run_calibrate`,
    /// which argot-core re-declares at its historical signature. The voice
    /// crate cannot call this build's composition root itself (that would be
    /// an illegal voice → core dependency cycle), so its `run_calibrate`
    /// takes the fit-time detectors as an explicit last parameter; this
    /// facade closes over `crate::compose::fit_detectors()` so every existing
    /// caller (the CLI, the bench harness, this crate's `tests/`) keeps
    /// compiling against the original argument list. The local `fn
    /// run_calibrate` below shadows the glob-imported one of the same name —
    /// ordinary Rust name resolution (an explicit item always wins over a
    /// glob import), not a redefinition error.
    pub mod calibration {
        pub use argot_rules_voice::scoring::calibration::*;

        pub fn run_calibrate(
            repo_dir: &std::path::Path,
            repo_corpus_path: &std::path::Path,
            generic_baseline_json: &[u8],
            output: &std::path::Path,
            opts: &CalibrateOptions,
        ) -> anyhow::Result<Vec<(String, f64)>> {
            argot_rules_voice::scoring::calibration::run_calibrate(
                repo_dir,
                repo_corpus_path,
                generic_baseline_json,
                output,
                opts,
                crate::compose::fit_detectors(),
            )
        }
    }

    // Language adapters and filters live in the leaf crate `argot-lang`;
    // re-exported here so `argot_core::scoring::adapters` /
    // `argot_core::scoring::filters` keep resolving at their original paths.
    pub use argot_lang::{adapters, filters};

    /// The structural-foreignness sense — node-kind bigrams as a repo's
    /// structural vocabulary, the shape analog of the foreign-vocabulary gate.
    /// Feature-gated (`--features structural`), advisory / measurement-only
    /// and pure-Rust (no new deps): absent and zero-cost off, and never wired
    /// into the base gating path. Lives in `argot-rules-voice` (the crate
    /// whose `scoring` module it extends).
    #[cfg(feature = "structural")]
    pub use argot_rules_voice::scoring::structural;

    /// The architecture-graph sense — a repo's module-dependency topology;
    /// flags an internal edge that reverses an established direction or
    /// leaves a sink layer (the relationship analog of the foreign-vocabulary
    /// gate). Feature-gated (`--features arch`), pure-Rust, lives in its own
    /// crate (`argot-rules-arch`) — absent and zero-cost off, never wired
    /// into the base gating path, base byte-unchanged. See the module docs +
    /// `docs/research/evidence/architecture-graph-foreignness.md`.
    #[cfg(feature = "arch")]
    pub use argot_rules_arch::graph as arch_graph;

    /// The test-integrity sense — per-version test inventories diffed into
    /// test-gaming events (rules `test-deleted` / `test-disabled` /
    /// `test-weakened`, group `integrity`). Feature-gated (`--features
    /// integrity`), pure-Rust, lives in its own crate (`argot-rules-integrity`):
    /// absent and zero-cost off, base byte-unchanged.
    #[cfg(feature = "integrity")]
    pub use argot_rules_integrity::model as integrity;
    #[cfg(feature = "integrity")]
    pub use argot_rules_integrity::test_inventory;

    /// The semantic layer — per-repo code embeddings powering the reinvention
    /// placement and nearest-code-evidence findings. Feature-gated, lives in
    /// its own crate (`argot-rules-semantic`): absent (and zero-cost) unless
    /// built with `--features semantic`.
    #[cfg(feature = "semantic")]
    pub use argot_rules_semantic as semantic;
}

// Language-substrate modules (adapters, tokenization, BPE, the dataset wire
// format) live in the leaf crate `argot-lang` and are re-exported here at
// their original paths so every existing `argot_core::bpe`/`dataset`/`text`/
// `tokenize` (and, via `scoring::mod`, `scoring::adapters`/`scoring::filters`)
// caller keeps compiling unchanged.
pub use argot_lang::{bpe, dataset, text, tokenize};
