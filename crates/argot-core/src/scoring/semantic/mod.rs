//! The semantic layer (feature `semantic`).
//!
//! argot's base guardrail is statistical — it judges *surface* markers (unknown
//! imports/callees, token surprise, convention rarity). The semantic layer adds
//! a second sense: a per-repo embedding index that asks *retrieval / local*
//! questions the surface scorers cannot:
//!
//! - **F1 reinvention** (`redundant`): does this new function duplicate one the
//!   repo already has? (nearest neighbour + margin)
//! - **F2 placement** (`misplaced`): do this function's nearest neighbours live
//!   where it's filed? (k-NN area voting)
//! - **F4 evidence**: show the closest existing code on any finding (retrieval).
//!
//! All three ride one shared [`index::SemanticIndex`] built at fit-time from the
//! repo's own functions and queried at check-time. The findings fire as the
//! `redundant`/`misplaced` rules (group `semantic`, configurable severity like
//! any rule) — real repos contain real duplication and cross-cutting helpers,
//! so the layer frames each finding with concrete evidence, and its numbers are
//! never folded into the base catch/false-alarm metric (embedding *novelty* is
//! deliberately NOT wired into the foreign-pattern scorers — see
//! `A3-F3-verdict.md`).
//!
//! Everything here is gated behind `feature = "semantic"`; with the feature off
//! the base statistical guardrail is byte-for-byte unchanged and pays no cost.

pub mod embedder;
pub mod index;
pub mod placement;
pub mod redundant;

/// The semantic index artifact filename, written next to `scorer-config.json`
/// in `.argot/` at fit and read back at check. Its own file keeps the base
/// config byte-for-byte unchanged.
pub const SEMANTIC_INDEX_FILE: &str = "semantic-index.json";
