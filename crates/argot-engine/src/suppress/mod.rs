//! Suppression surfaces — "argot, you're wrong about this hunk, stop
//! flagging it".
//!
//! Three user-facing surfaces resolve into one decision at check time:
//!
//! 1. **`argot.toml` `[exclude]`** (repo root) — the built-in `argot:recommended`
//!    toggle plus `paths`, gitignore-style patterns
//!    ([`path_rules::PathSuppressions`], fed by [`crate::config::ArgotConfig`]).
//!    Calibration sampling, the check scope filter, and `argot inspect` all
//!    consult the same resolved set (lock-step principle).
//! 2. **Inline magic comments** — `# argot: ignore-next-line — <reason>` and
//!    block variants, language-aware via the adapters' line-comment prefix
//!    ([`inline`]).
//! 3. **`argot.toml` `[[mute]]`** — durable rules with optional rule /
//!    hit-hash / expiry scoping ([`rules_file`]); `argot mute` appends
//!    hash-scoped entries ([`mute`]) resolved via the last-check cache
//!    ([`last_check`]).
//!
//! Suppressed ≠ deleted: check drops suppressed hits from output and exit-code
//! consideration but reports a one-line count summary on stderr.
//!
//! `ignore_suggest` (the `argot init --suggest` candidate scan) stays in
//! `argot-core`, not here: it classifies files by [`crate`]-external language
//! machinery (`inspect::adapter_for`, `scoring::calibration::language_for_filename`)
//! that is itself downstream of this rule-agnostic engine, so it cannot live
//! on this side without an illegal engine → core dependency. `argot_core::suppress`
//! re-exports it at its historical path.

pub mod classify;
pub mod glob;
pub mod hit_hash;
pub mod inline;
pub mod last_check;
pub mod mute;
pub mod path_rules;
pub mod rules_file;

pub use classify::FileSuppressions;
pub use glob::fnmatch;
pub use hit_hash::hit_hash;
pub use inline::{parse_inline, InlineRule, InlineSuppressions, InlineWarning};
pub use last_check::{read_last_check, write_last_check, LastCheckHit, LAST_CHECK_FILE};
pub use mute::{mute_hash, mute_path, DEFAULT_MUTE_REASON};
pub use path_rules::{recommended_excluded, rel_string, PathScope, PathSuppressions};
pub use rules_file::{build_mutes, RawMute, SuppressionRule, SuppressionsFile};
