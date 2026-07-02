//! Suppression surfaces — "argot, you're wrong about this hunk, stop
//! flagging it".
//!
//! Three user-facing surfaces resolve into one decision at check time:
//!
//! 1. **`.argotignore`** (repo root) — gitignore-style path patterns layered
//!    on top of the built-in `argot:recommended` set
//!    ([`path_rules::PathSuppressions`]). Calibration sampling, the check
//!    scope filter, and `argot inspect` all consult the same resolved set
//!    (lock-step principle).
//! 2. **Inline magic comments** — `# argot: ignore-next-line — <reason>` and
//!    block variants, language-aware via the adapters' line-comment prefix
//!    ([`inline`]).
//! 3. **`.argot/suppressions.yaml`** — durable rules with optional scorer /
//!    hit-hash / expiry scoping ([`rules_file`]); `argot mute` appends
//!    hash-scoped entries ([`mute`]) resolved via the last-check cache
//!    ([`last_check`]).
//!
//! Suppressed ≠ deleted: check drops suppressed hits from output and exit-code
//! consideration but reports a one-line count summary on stderr.

pub mod glob;
pub mod hit_hash;
pub mod inline;
pub mod last_check;
pub mod mute;
pub mod path_rules;
pub mod rules_file;

pub use glob::fnmatch;
pub use hit_hash::hit_hash;
pub use inline::{parse_inline, InlineRule, InlineSuppressions, InlineWarning};
pub use last_check::{read_last_check, write_last_check, LastCheckHit, LAST_CHECK_FILE};
pub use mute::{mute_hash, DEFAULT_MUTE_REASON, SUPPRESSIONS_FILE};
pub use path_rules::{recommended_excluded, rel_string, PathScope, PathSuppressions};
pub use rules_file::{load_suppressions_file, SuppressionRule, SuppressionsFile};
