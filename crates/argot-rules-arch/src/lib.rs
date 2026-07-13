//! The architecture rule group — the relationship analog of the base
//! foreign-vocabulary gate.
//!
//! The base gate catches a foreign **dependency** (an external import 0-usage
//! in the repo). This crate catches a foreign **relationship**: an *internal*
//! module-dependency edge the repo's own topology never has — a layer it
//! never crosses, or a dependency **direction** it never uses. See
//! [`graph`]'s module docs for the fire rule and the language-resolver
//! matrix.
//!
//! No cargo features: this whole crate IS the `arch` feature — argot-core
//! compiles it in only when built with `--features arch` (an optional
//! dependency), and it never depends on argot-core, so it stays usable
//! standalone and cannot cycle back.

pub mod detector;
pub mod graph;

pub use detector::ArchDetector;
