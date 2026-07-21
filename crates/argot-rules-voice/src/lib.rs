//! The voice rule group — argot's base statistical guardrail.
//!
//! The base guardrail pipeline: git walk → tokenize → score (BPE surprise,
//! call-receiver, conventions, typicality) → calibrate → check. Unlike the
//! additive
//! `argot-rules-{semantic,arch,integrity}` slices, this crate is not a cargo
//! feature — argot-core's composition root ([`compose`](../argot_core/index.html))
//! wires [`VoiceDetector`] into the engine's `check` loop unconditionally; the
//! base voice model is always fitted and always checked.
//!
//! No cargo features of its own besides `structural` (an advisory,
//! non-gating research sense kept off in releases — see
//! [`scoring::structural`]).
//!
//! This crate never depends on `argot-core` or the other rule-group crates,
//! so it stays usable standalone and cannot cycle back. `run_calibrate`
//! ([`scoring::calibration::run_calibrate`]) cannot call argot-core's
//! composition root directly (that would be the cycle); instead it takes the
//! build's fit-time detectors as its last parameter, supplied by the caller
//! (argot-core's facade closes over its own `compose::fit_detectors()`).

pub mod convention_catalog;
pub mod detector;
pub mod extract;
pub mod ignore_suggest;
pub mod inspect;
pub mod load;
pub mod placement;
pub mod scoring;
pub mod train;

pub use detector::VoiceDetector;
