//! The test-integrity rule group — catches an agent gaming tests to green a
//! failing suite: weakening, disabling, or deleting them **alongside a
//! production change**. See [`model`]'s module docs for the event taxonomy
//! and the FP-first design gate.
//!
//! No cargo features: this whole crate IS the `integrity` feature —
//! argot-core compiles it in only when built with `--features integrity` (an
//! optional dependency), and it never depends on argot-core, so it stays
//! usable standalone and cannot cycle back.

pub mod detector;
pub mod model;
pub mod test_inventory;

pub use detector::IntegrityDetector;
