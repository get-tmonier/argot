//! Durable per-hit mutes — the `[[mute]]` tables in `argot.toml`.
//!
//! Each `[[mute]]` accepts a specific hit (or a path glob) into the repo's
//! voice permanently, as a committed audit trail:
//!
//! ```toml
//! [[mute]]
//! path = "src/vendored/**"     # mandatory glob (fnmatch, `*` crosses `/`)
//! scorer = "bpe"               # optional: bpe | import | call_receiver
//! hash = "a1b2c3d4e5f6"        # optional: hit hash (`argot mute` writes these)
//! expires = "2026-12-31"       # optional: YYYY-MM-DD, ignored past this date
//! reason = "vendored upstream" # mandatory
//! ```
//!
//! This module owns the mute *type* and its validation; `config.rs` does the
//! TOML I/O and hands the raw tables here via [`build_mutes`]. Expiry never
//! calls system time — callers pass `today` as an ISO `YYYY-MM-DD` string (ISO
//! dates compare correctly as strings), so the CLI passes the real date and
//! tests pass fixed ones.

use crate::suppress::glob::fnmatch;
use serde::{Deserialize, Serialize};

/// One validated, active mute.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuppressionRule {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scorer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
    pub reason: String,
}

impl SuppressionRule {
    /// Does this rule suppress a hit at `rel_path` with the given winning
    /// reason code and hit hash?
    pub fn matches(&self, rel_path: &str, reason_code: &str, hit_hash: &str) -> bool {
        fnmatch(rel_path, &self.path)
            && self.scorer.as_deref().is_none_or(|s| s == reason_code)
            && self.hash.as_deref().is_none_or(|h| h == hit_hash)
    }
}

/// Permissive on-disk shape: every field optional so one malformed `[[mute]]`
/// is a warning, not a parse failure for the whole config.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawMute {
    pub path: Option<String>,
    pub scorer: Option<String>,
    pub hash: Option<String>,
    pub expires: Option<String>,
    pub reason: Option<String>,
}

/// The resolved mute set, split into active and expired rules.
#[derive(Debug, Clone, Default)]
pub struct SuppressionsFile {
    pub active: Vec<SuppressionRule>,
    pub expired: Vec<SuppressionRule>,
    /// Human-readable notes (expired entries, invalid entries) for stderr.
    pub warnings: Vec<String>,
}

fn valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| matches!(i, 4 | 7) || c.is_ascii_digit())
}

/// Validate raw `[[mute]]` tables into active/expired rules. `today` is
/// `YYYY-MM-DD`; entries with `expires < today` land in `expired` with a note.
/// A malformed single entry is skipped with a warning, never a hard error.
pub fn build_mutes(raw: Vec<RawMute>, today: &str) -> SuppressionsFile {
    let mut out = SuppressionsFile::default();
    for (i, r) in raw.into_iter().enumerate() {
        let label = r
            .path
            .clone()
            .or_else(|| r.hash.clone())
            .unwrap_or_else(|| format!("entry {}", i + 1));
        let Some(path) = r.path else {
            out.warnings.push(format!(
                "argot.toml: mute {label}: missing mandatory 'path' — entry ignored"
            ));
            continue;
        };
        let Some(reason) = r.reason.filter(|s| !s.trim().is_empty()) else {
            out.warnings.push(format!(
                "argot.toml: mute {label}: missing mandatory 'reason' — entry ignored"
            ));
            continue;
        };
        if let Some(scorer) = &r.scorer {
            if !crate::suppress::inline::KNOWN_SCORERS.contains(&scorer.as_str()) {
                out.warnings.push(format!(
                    "argot.toml: mute {label}: unknown scorer '{scorer}' — entry ignored"
                ));
                continue;
            }
        }
        if let Some(expires) = &r.expires {
            if !valid_date(expires) {
                out.warnings.push(format!(
                    "argot.toml: mute {label}: invalid expires '{expires}' \
                     (expected YYYY-MM-DD) — entry ignored"
                ));
                continue;
            }
        }
        let rule = SuppressionRule {
            path,
            scorer: r.scorer,
            hash: r.hash,
            expires: r.expires,
            reason,
        };
        let expired = rule.expires.as_deref().is_some_and(|e| e < today);
        if expired {
            out.warnings.push(format!(
                "argot.toml: mute {label}: expired {} — ignored",
                rule.expires.as_deref().unwrap_or("")
            ));
            out.expired.push(rule);
        } else {
            out.active.push(rule);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: &str = "2026-07-02";

    fn raw(
        path: Option<&str>,
        scorer: Option<&str>,
        hash: Option<&str>,
        expires: Option<&str>,
        reason: Option<&str>,
    ) -> RawMute {
        RawMute {
            path: path.map(str::to_string),
            scorer: scorer.map(str::to_string),
            hash: hash.map(str::to_string),
            expires: expires.map(str::to_string),
            reason: reason.map(str::to_string),
        }
    }

    #[test]
    fn build_valid_mutes() {
        let f = build_mutes(
            vec![
                raw(
                    Some("src/vendored/**"),
                    None,
                    None,
                    None,
                    Some("vendored upstream"),
                ),
                raw(
                    Some("app.py"),
                    Some("bpe"),
                    Some("a1b2c3d4e5f6"),
                    Some("2099-01-01"),
                    Some("accepted oddity"),
                ),
            ],
            TODAY,
        );
        assert_eq!(f.active.len(), 2);
        assert!(f.expired.is_empty());
        assert!(f.warnings.is_empty());
        assert_eq!(f.active[0].path, "src/vendored/**");
        assert_eq!(f.active[1].scorer.as_deref(), Some("bpe"));
        assert_eq!(f.active[1].hash.as_deref(), Some("a1b2c3d4e5f6"));
    }

    #[test]
    fn expired_entry_is_ignored_with_note() {
        let f = build_mutes(
            vec![
                raw(
                    Some("app.py"),
                    None,
                    None,
                    Some("2026-07-01"),
                    Some("short-lived"),
                ),
                raw(
                    Some("keep.py"),
                    None,
                    None,
                    Some("2026-07-02"),
                    Some("expires today, still active"),
                ),
            ],
            TODAY,
        );
        assert_eq!(f.active.len(), 1, "expires == today is still active");
        assert_eq!(f.active[0].path, "keep.py");
        assert_eq!(f.expired.len(), 1);
        assert!(f.warnings.iter().any(|w| w.contains("expired 2026-07-01")));
    }

    #[test]
    fn missing_reason_entry_skipped_with_warning() {
        let f = build_mutes(
            vec![
                raw(Some("app.py"), None, None, None, None),
                raw(Some("ok.py"), None, None, None, Some("fine")),
            ],
            TODAY,
        );
        assert_eq!(f.active.len(), 1);
        assert!(f
            .warnings
            .iter()
            .any(|w| w.contains("missing mandatory 'reason'")));
    }

    #[test]
    fn invalid_date_and_scorer_are_rejected() {
        let f = build_mutes(
            vec![
                raw(Some("a.py"), None, None, Some("soonish"), Some("r")),
                raw(Some("b.py"), Some("quantum"), None, None, Some("r")),
            ],
            TODAY,
        );
        assert!(f.active.is_empty());
        assert!(f
            .warnings
            .iter()
            .any(|w| w.contains("invalid expires 'soonish'")));
        assert!(f
            .warnings
            .iter()
            .any(|w| w.contains("unknown scorer 'quantum'")));
    }

    #[test]
    fn missing_path_entry_skipped() {
        let f = build_mutes(
            vec![raw(None, None, Some("abc123"), None, Some("r"))],
            TODAY,
        );
        assert!(f.active.is_empty());
        assert!(f
            .warnings
            .iter()
            .any(|w| w.contains("missing mandatory 'path'")));
    }

    #[test]
    fn matches_by_glob_scorer_and_hash() {
        let rule = SuppressionRule {
            path: "src/*.py".to_string(),
            scorer: Some("bpe".to_string()),
            hash: Some("abc123def456".to_string()),
            expires: None,
            reason: "r".to_string(),
        };
        assert!(rule.matches("src/deep/x.py", "bpe", "abc123def456"));
        assert!(!rule.matches("src/x.py", "import", "abc123def456"));
        assert!(!rule.matches("src/x.py", "bpe", "otherhash0000"));
        assert!(!rule.matches("lib/x.py", "bpe", "abc123def456"));

        let broad = SuppressionRule {
            path: "src/**".to_string(),
            scorer: None,
            hash: None,
            expires: None,
            reason: "r".to_string(),
        };
        assert!(broad.matches("src/anything.ts", "call_receiver", "whatever12345"));
    }
}
