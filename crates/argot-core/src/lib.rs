//! argot-core — the voice-linter engine.
//!
//! This crate is a behaviour-preserving port of the Python engine
//! (`engine/argot`). It is deliberately language- and corpus-agnostic: no
//! hardcoded framework/language literals leak into scoring logic (per the
//! project's `CLAUDE.md`). Framework-specific knowledge lives only in
//! fixtures, benchmarks, and eval code — never here.
//!
//! Pipeline: extract → train → calibrate → check.

pub mod cache;
pub mod check;
pub mod config;
pub(crate) mod detector;
pub mod extract;
pub mod finding;
pub mod git_walk;
pub mod health;
pub mod inspect;
pub mod json;
pub mod output;
#[cfg(any(feature = "semantic", feature = "integrity"))]
pub mod par;
pub mod rules;
pub mod scoring;
pub mod stats;
pub mod suppress;
pub mod timing;
pub mod train;

// Language-substrate modules (adapters, tokenization, BPE, the dataset wire
// format) live in the leaf crate `argot-lang` and are re-exported at their
// original paths so every existing `argot_core::bpe`/`dataset`/`text`/
// `tokenize` (and, via `scoring::mod`, `scoring::adapters`/`scoring::filters`)
// caller keeps compiling unchanged.
pub use argot_lang::{bpe, dataset, text, tokenize};
