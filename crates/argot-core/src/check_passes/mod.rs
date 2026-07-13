//! The feature-gated rule-group passes — the [`argot_engine::detector::Detector`]
//! implementations this crate plugs into the rule-agnostic engine's `check`
//! loop. Named `check_passes` (not `check`, which is the crate-root facade
//! over `argot_engine::check` — see `crate::compose` for the wiring) so each
//! pass can move to its own crate later without a further rename.

pub(crate) mod load;
pub(crate) mod voice;

#[cfg(feature = "arch")]
pub(crate) mod arch;
#[cfg(feature = "integrity")]
pub(crate) mod integrity;
#[cfg(feature = "semantic")]
pub(crate) mod semantic;

#[cfg(test)]
mod tests;

pub use load::RepoScorers;
