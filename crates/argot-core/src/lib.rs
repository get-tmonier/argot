//! argot-core — the voice-linter engine.
//!
//! This crate is a behaviour-preserving port of the Python engine
//! (`engine/argot`). It is deliberately language- and corpus-agnostic: no
//! hardcoded framework/language literals leak into scoring logic (per the
//! project's `CLAUDE.md`). Framework-specific knowledge lives only in
//! fixtures, benchmarks, and eval code — never here.
//!
//! Pipeline: extract → train → calibrate → check.

pub mod bpe;
pub mod check;
pub mod dataset;
pub mod extract;
pub mod git_walk;
pub mod inspect;
pub mod json;
pub mod output;
pub mod scoring;
pub mod stats;
pub mod text;
pub mod tokenize;
pub mod train;
