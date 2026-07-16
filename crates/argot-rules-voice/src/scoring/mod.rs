//! Scoring engine.
//!
//! The scoring surface: BPE surprise scoring, the import-graph and
//! call-receiver sub-scorers, language adapters, typicality/data-dominant
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
// `adapters`/`filters` were `pub`, `ts_parse` was `pub(crate)`). Only the
// re-exports the voice slice itself uses land here — the semantic/arch/
// integrity facade re-exports stay in argot-core's own `scoring` facade
// (this crate never depends on those slice crates, so it cannot re-export
// them without creating the dependency).
pub use argot_lang::adapters;
pub use argot_lang::filters;
pub(crate) use argot_lang::ts_parse;
