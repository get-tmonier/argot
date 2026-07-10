//! Scoring engine — port of `engine/argot/scoring`.
//!
//! Mirrors the Python package layout: BPE surprise scoring, the import-graph
//! and call-receiver sub-scorers, language adapters, typicality/data-dominant
//! filters, calibration, and evidence. The production composite is
//! `sequential::SequentialImportBpeScorer`.

pub mod adapters;
/// The architecture-graph sense — a repo's module-dependency topology; flags an
/// internal edge that reverses an established direction or leaves a sink layer
/// (the relationship analog of the foreign-vocabulary gate). Feature-gated
/// (`--features arch`), pure-Rust — emits the `layering` rule: absent and
/// zero-cost off, never wired into the base gating path, base byte-unchanged.
/// See the module docs + `docs/research/evidence/architecture-graph-foreignness.md`.
#[cfg(feature = "arch")]
pub mod arch_graph;
pub mod bpe_scorer;
pub mod calibration;
pub mod call_receiver;
pub mod conventions;
pub mod evidence;
pub mod filters;
pub mod import_graph;
pub mod model;
/// The semantic layer — per-repo code embeddings powering the reinvention
/// placement and nearest-code-evidence findings. Feature-gated: absent (and
/// zero-cost) unless built with `--features semantic`.
#[cfg(feature = "semantic")]
pub mod semantic;
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
pub(crate) mod ts_parse;
