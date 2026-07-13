//! argot-engine — the rule-agnostic guardrail engine.
//!
//! Extracted from `argot-core` (see `docs/rust-port/` and the workspace
//! `CLAUDE.md`): the [`detector`] contract a rule group plugs into, `check`
//! orchestration ([`check`]), config ([`config`]), the suppression surfaces
//! ([`suppress`]), git/corpus walking ([`git_walk`], [`corpus`]), and the
//! shared rendering/registry/reporting plumbing ([`output`], [`rules`],
//! [`finding`]). It knows about zero rule groups by name — which
//! [`detector::Detector`]s a build wires in is decided one layer up, by
//! `argot-core`'s composition root (`compose::default_detectors`), passed
//! into [`check::run_check`] as a plain `Vec`.
//!
//! No cargo features: this crate is the same for every build. The
//! feature-gated rule groups (semantic/architecture/integrity) and the base
//! statistical scorers live in `argot-core`, which depends on this crate —
//! never the other way around.

pub mod artifact;
pub mod cache;
pub mod check;
pub mod config;
pub mod corpus;
pub mod detector;
pub mod finding;
pub mod git_walk;
pub mod health;
pub mod json;
pub mod output;
pub mod par;
pub mod rules;
pub mod stats;
pub mod suppress;
pub mod timing;

// Language-substrate modules (adapters, tokenization, BPE, the dataset wire
// format) live in the leaf crate `argot-lang` and are re-exported here at
// their historical `argot_core`/`argot_engine` path so existing callers keep
// compiling unchanged.
pub use argot_lang::text;
