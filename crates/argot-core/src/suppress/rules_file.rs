//! `.argot/suppressions.yaml` — the durable, richer suppression surface.
//!
//! A top-level YAML list of entries:
//!
//! ```yaml
//! - path: "src/vendored/**"     # mandatory glob (fnmatch, `*` crosses `/`)
//!   scorer: bpe                 # optional: bpe | import | call_receiver
//!   hash: a1b2c3d4e5f6          # optional: hit hash (`argot mute` writes these)
//!   expires: 2026-12-31         # optional: YYYY-MM-DD, ignored past this date
//!   reason: vendored upstream   # mandatory
//! ```
//!
//! Expiry never calls system time in core logic — callers pass `today` as an
//! ISO `YYYY-MM-DD` string (ISO dates compare correctly as strings), so the
//! CLI passes the real date and tests pass fixed ones.

use crate::suppress::glob::fnmatch;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One validated, active suppression rule.
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

/// Permissive on-disk shape: every field optional so one malformed entry is a
/// warning, not a parse failure for the whole file.
#[derive(Debug, Clone, Deserialize)]
struct RawRule {
    path: Option<String>,
    scorer: Option<String>,
    hash: Option<String>,
    expires: Option<String>,
    reason: Option<String>,
}

/// The loaded suppressions file, split into active and expired rules.
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

/// Load and validate `path` (absent file → empty set). `today` is
/// `YYYY-MM-DD`; entries with `expires < today` land in `expired` with a note.
pub fn load_suppressions_file(path: &Path, today: &str) -> SuppressionsFile {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return SuppressionsFile::default(),
    };
    parse_suppressions(&content, today)
}

/// Parse suppressions.yaml content — see [`load_suppressions_file`].
pub fn parse_suppressions(content: &str, today: &str) -> SuppressionsFile {
    let mut out = SuppressionsFile::default();
    if content.trim().is_empty() {
        return out;
    }
    let raw: Vec<RawRule> = match serde_yaml::from_str(content) {
        Ok(r) => r,
        Err(e) => {
            out.warnings.push(format!(
                "suppressions.yaml: not a valid rule list — ignored ({e})"
            ));
            return out;
        }
    };
    for (i, r) in raw.into_iter().enumerate() {
        let label = r
            .path
            .clone()
            .or_else(|| r.hash.clone())
            .unwrap_or_else(|| format!("entry {}", i + 1));
        let Some(path) = r.path else {
            out.warnings.push(format!(
                "suppressions.yaml: {label}: missing mandatory 'path' — entry ignored"
            ));
            continue;
        };
        let Some(reason) = r.reason.filter(|s| !s.trim().is_empty()) else {
            out.warnings.push(format!(
                "suppressions.yaml: {label}: missing mandatory 'reason' — entry ignored"
            ));
            continue;
        };
        if let Some(scorer) = &r.scorer {
            if !crate::suppress::inline::KNOWN_SCORERS.contains(&scorer.as_str()) {
                out.warnings.push(format!(
                    "suppressions.yaml: {label}: unknown scorer '{scorer}' — entry ignored"
                ));
                continue;
            }
        }
        if let Some(expires) = &r.expires {
            if !valid_date(expires) {
                out.warnings.push(format!(
                    "suppressions.yaml: {label}: invalid expires '{expires}' \
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
                "suppressions.yaml: {label}: expired {} — ignored",
                rule.expires.as_deref().unwrap_or("")
            ));
            out.expired.push(rule);
        } else {
            out.active.push(rule);
        }
    }
    out
}

/// Serialize rules back to YAML (used by `review-mutes --prune`).
pub fn serialize_rules(rules: &[SuppressionRule]) -> String {
    if rules.is_empty() {
        return String::new();
    }
    serde_yaml::to_string(rules).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: &str = "2026-07-02";

    #[test]
    fn load_valid_yaml() {
        let yaml = "\
- path: \"src/vendored/**\"
  reason: vendored upstream
- path: app.py
  scorer: bpe
  hash: a1b2c3d4e5f6
  expires: 2099-01-01
  reason: accepted oddity
";
        let f = parse_suppressions(yaml, TODAY);
        assert_eq!(f.active.len(), 2);
        assert!(f.expired.is_empty());
        assert!(f.warnings.is_empty());
        assert_eq!(f.active[0].path, "src/vendored/**");
        assert_eq!(f.active[1].scorer.as_deref(), Some("bpe"));
        assert_eq!(f.active[1].hash.as_deref(), Some("a1b2c3d4e5f6"));
    }

    #[test]
    fn expired_entry_is_ignored_with_note() {
        let yaml = "\
- path: app.py
  expires: 2026-07-01
  reason: short-lived
- path: keep.py
  expires: 2026-07-02
  reason: expires today, still active
";
        let f = parse_suppressions(yaml, TODAY);
        assert_eq!(f.active.len(), 1, "expires == today is still active");
        assert_eq!(f.active[0].path, "keep.py");
        assert_eq!(f.expired.len(), 1);
        assert!(f.warnings.iter().any(|w| w.contains("expired 2026-07-01")));
    }

    #[test]
    fn missing_reason_entry_skipped_with_warning() {
        let yaml = "- path: app.py\n- path: ok.py\n  reason: fine\n";
        let f = parse_suppressions(yaml, TODAY);
        assert_eq!(f.active.len(), 1);
        assert!(f
            .warnings
            .iter()
            .any(|w| w.contains("missing mandatory 'reason'")));
    }

    #[test]
    fn invalid_date_and_scorer_are_rejected() {
        let yaml = "\
- path: a.py
  expires: soonish
  reason: r
- path: b.py
  scorer: quantum
  reason: r
";
        let f = parse_suppressions(yaml, TODAY);
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
    fn absent_file_is_empty() {
        let f = load_suppressions_file(Path::new("/nonexistent/suppressions.yaml"), TODAY);
        assert!(f.active.is_empty() && f.expired.is_empty() && f.warnings.is_empty());
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

    #[test]
    fn serialize_roundtrips() {
        let rules = vec![SuppressionRule {
            path: "a.py".to_string(),
            scorer: None,
            hash: Some("abc123def456".to_string()),
            expires: Some("2099-01-01".to_string()),
            reason: "muted via argot mute".to_string(),
        }];
        let yaml = serialize_rules(&rules);
        let f = parse_suppressions(&yaml, TODAY);
        assert_eq!(f.active, rules);
    }
}
