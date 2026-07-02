//! `argot mute <hit-hash>` — append a hash-scoped rule to
//! `.argot/suppressions.yaml`, resolving the hash against the last check run's
//! cache (`.argot/last-check.json`).
//!
//! The append is textual (one serialized list item) so hand-edited YAML —
//! comments included — is never rewritten by a mute.

use crate::suppress::last_check::read_last_check;
use crate::suppress::rules_file::{load_suppressions_file, SuppressionRule};
use std::path::Path;

/// File name of the rules file inside the argot dir.
pub const SUPPRESSIONS_FILE: &str = "suppressions.yaml";

/// Default reason recorded when `argot mute` is run without `--reason`.
pub const DEFAULT_MUTE_REASON: &str = "muted via argot mute";

/// Append a hash-scoped suppression for `hash`. `expires` is an optional
/// `YYYY-MM-DD` date (already resolved by the caller); `today` gates the
/// duplicate check against active rules. Returns the written rule.
pub fn mute_hash(
    argot_dir: &Path,
    hash: &str,
    reason: Option<&str>,
    expires: Option<String>,
    today: &str,
) -> Result<SuppressionRule, String> {
    let hits = read_last_check(argot_dir).ok_or_else(|| {
        format!(
            "no check results found ({}/last-check.json) — run `argot check` first",
            argot_dir.display()
        )
    })?;
    let hit = hits.iter().find(|h| h.hash == hash).ok_or_else(|| {
        format!("hit hash '{hash}' not found in the last check results — run `argot check` and copy a [hash] from a hit")
    })?;

    let rules_path = argot_dir.join(SUPPRESSIONS_FILE);
    let existing = load_suppressions_file(&rules_path, today);
    if existing
        .active
        .iter()
        .any(|r| r.hash.as_deref() == Some(hash))
    {
        return Err(format!(
            "hit '{hash}' is already muted in {SUPPRESSIONS_FILE}"
        ));
    }

    let rule = SuppressionRule {
        path: hit.path.clone(),
        scorer: None,
        hash: Some(hash.to_string()),
        expires,
        reason: reason
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .unwrap_or(DEFAULT_MUTE_REASON)
            .to_string(),
    };

    let item = serde_yaml::to_string(std::slice::from_ref(&rule))
        .map_err(|e| format!("failed to serialize suppression entry: {e}"))?;
    let mut content = std::fs::read_to_string(&rules_path).unwrap_or_default();
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&item);
    std::fs::create_dir_all(argot_dir)
        .map_err(|e| format!("cannot create {}: {e}", argot_dir.display()))?;
    std::fs::write(&rules_path, content)
        .map_err(|e| format!("cannot write {}: {e}", rules_path.display()))?;
    Ok(rule)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suppress::last_check::{write_last_check, LastCheckHit};
    use std::path::PathBuf;

    const TODAY: &str = "2026-07-02";

    fn argot_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("argot_mute_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn seed_last_check(dir: &Path) {
        write_last_check(
            dir,
            &[LastCheckHit {
                path: "src/app.py".to_string(),
                reason: "bpe".to_string(),
                hash: "abc123def456".to_string(),
                line_start: 1,
                line_end: 9,
            }],
        )
        .unwrap();
    }

    #[test]
    fn mute_appends_hash_scoped_rule_with_default_reason() {
        let dir = argot_dir("append");
        seed_last_check(&dir);
        let rule = mute_hash(&dir, "abc123def456", None, None, TODAY).unwrap();
        assert_eq!(rule.path, "src/app.py");
        assert_eq!(rule.hash.as_deref(), Some("abc123def456"));
        assert_eq!(rule.reason, DEFAULT_MUTE_REASON);

        let loaded = load_suppressions_file(&dir.join(SUPPRESSIONS_FILE), TODAY);
        assert_eq!(loaded.active, vec![rule]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mute_preserves_existing_file_content_textually() {
        let dir = argot_dir("preserve");
        seed_last_check(&dir);
        std::fs::write(
            dir.join(SUPPRESSIONS_FILE),
            "# hand-written\n- path: keep.py\n  reason: manual rule\n",
        )
        .unwrap();
        mute_hash(
            &dir,
            "abc123def456",
            Some("noisy vendored hunk"),
            None,
            TODAY,
        )
        .unwrap();
        let content = std::fs::read_to_string(dir.join(SUPPRESSIONS_FILE)).unwrap();
        assert!(content.starts_with("# hand-written\n"), "comment preserved");
        assert!(content.contains("keep.py"));
        assert!(content.contains("noisy vendored hunk"));
        let loaded = load_suppressions_file(&dir.join(SUPPRESSIONS_FILE), TODAY);
        assert_eq!(loaded.active.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mute_unknown_hash_errors() {
        let dir = argot_dir("unknown");
        seed_last_check(&dir);
        let err = mute_hash(&dir, "000000000000", None, None, TODAY).unwrap_err();
        assert!(err.contains("not found in the last check results"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mute_without_last_check_errors() {
        let dir = argot_dir("nocache");
        let err = mute_hash(&dir, "abc123def456", None, None, TODAY).unwrap_err();
        assert!(err.contains("run `argot check` first"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn duplicate_mute_errors() {
        let dir = argot_dir("dup");
        seed_last_check(&dir);
        mute_hash(&dir, "abc123def456", None, None, TODAY).unwrap();
        let err = mute_hash(&dir, "abc123def456", None, None, TODAY).unwrap_err();
        assert!(err.contains("already muted"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mute_with_expiry_records_date() {
        let dir = argot_dir("expiry");
        seed_last_check(&dir);
        let rule = mute_hash(
            &dir,
            "abc123def456",
            None,
            Some("2026-08-01".to_string()),
            TODAY,
        )
        .unwrap();
        assert_eq!(rule.expires.as_deref(), Some("2026-08-01"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
