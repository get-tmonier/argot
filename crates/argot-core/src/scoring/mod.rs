//! Scoring engine — port of `engine/argot/scoring`.
//!
//! Mirrors the Python package layout: BPE surprise scoring, the import-graph
//! and call-receiver sub-scorers, language adapters, typicality/data-dominant
//! filters, calibration, and evidence. The production composite is
//! `sequential::SequentialImportBpeScorer`.

pub mod bpe_scorer;
pub mod calibration;
pub mod call_receiver;
pub mod conventions;
pub mod evidence;
pub mod import_graph;
pub mod model;
pub mod sequential;
pub mod shape_primitive;
pub mod shape_primitives;
/// The structural-foreignness sense — node-kind bigrams as a repo's structural
/// vocabulary, the shape analog of the foreign-vocabulary gate. Feature-gated
/// (`--features structural`), advisory / measurement-only and pure-Rust (no new
/// deps): absent and zero-cost off, and never wired into the base gating path,
/// so the shipped guardrail is byte-for-byte unchanged. See the module docs and
/// `docs/research/evidence/foreign-structure-gate-floor.md`.
#[cfg(feature = "structural")]
pub mod structural;
pub mod typicality;

mod minhash_params_seed0;
pub(crate) mod numpy_sampler;

// Language adapters, filters, and the reused-parser helper live in the leaf
// crate `argot-lang`; re-exported here so `crate::scoring::adapters` /
// `crate::scoring::filters` / `crate::scoring::ts_parse` keep resolving at
// their original paths (visibility matches what each was before the move:
// `adapters`/`filters` were `pub`, `ts_parse` was `pub(crate)`).
pub use argot_lang::adapters;
pub use argot_lang::filters;
pub(crate) use argot_lang::ts_parse;

/// The architecture-graph sense — a repo's module-dependency topology; flags an
/// internal edge that reverses an established direction or leaves a sink layer
/// (the relationship analog of the foreign-vocabulary gate). Feature-gated
/// (`--features arch`), pure-Rust, lives in its own crate (`argot-rules-arch`)
/// — absent and zero-cost off, never wired into the base gating path, base
/// byte-unchanged. See the module docs +
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
/// placement and nearest-code-evidence findings. Feature-gated, lives in its
/// own crate (`argot-rules-semantic`): absent (and zero-cost) unless built
/// with `--features semantic`.
#[cfg(feature = "semantic")]
pub use argot_rules_semantic as semantic;
