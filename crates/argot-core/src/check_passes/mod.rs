//! The base (voice) rule-group pass — the [`argot_engine::detector::Detector`]
//! implementation this crate plugs into the rule-agnostic engine's `check`
//! loop. Named `check_passes` (not `check`, which is the crate-root facade
//! over `argot_engine::check` — see `crate::compose` for the wiring). The
//! additive rule groups (semantic/architecture/integrity) have each moved to
//! their own crate (`argot-rules-semantic`/`argot-rules-arch`/
//! `argot-rules-integrity`), wired in as optional dependencies behind the
//! matching cargo feature — see `crate::compose`.

pub(crate) mod load;
pub(crate) mod voice;

pub use load::RepoScorers;
