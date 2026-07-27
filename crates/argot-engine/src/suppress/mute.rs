//! `argot mute <hit-hash>` — append a hash-scoped `[[mute]]` to `argot.toml`,
//! resolving the hash against the last check run's cache
//! (`.argot/last-check.json`).
//!
//! The append is a format-preserving TOML edit ([`crate::config::append_mute`]),
//! so hand-edited config — comments included — is never rewritten by a mute.

use crate::config::{append_mute, ArgotConfig};
use crate::suppress::last_check::read_last_check;
use crate::suppress::rules_file::SuppressionRule;
use std::path::Path;

/// Default reason recorded when `argot mute` is run without `--reason`.
pub const DEFAULT_MUTE_REASON: &str = "muted via argot mute";

/// Append a hash-scoped mute for `hash` to `<repo_root>/argot.toml`. The hit's
/// path/hash come from the last check cached under `argot_dir`. `expires` is an
/// optional `YYYY-MM-DD` date (already resolved by the caller); `today` gates
/// the duplicate check against active rules. Returns the written rule.
pub fn mute_hash(
    repo_root: &Path,
    argot_dir: &Path,
    registry: &crate::rules::Registry,
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

    let config = ArgotConfig::load_with(repo_root, registry);
    if config
        .mutes_with(registry, today)
        .active
        .iter()
        .any(|r| r.hash.as_deref() == Some(hash))
    {
        return Err(format!("hit '{hash}' is already muted in argot.toml"));
    }

    let rule = SuppressionRule {
        path: hit.path.clone(),
        rule: None,
        hash: Some(hash.to_string()),
        expires,
        reason: reason
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .unwrap_or(DEFAULT_MUTE_REASON)
            .to_string(),
    };

    append_mute(repo_root, &rule)?;
    Ok(rule)
}

/// Append a durable, pattern-scoped `[[mute]]` — the form that survives a
/// re-run, because it names a path shape rather than one hit's hash.
///
/// `rule` narrows it to a rule or group; it is validated against `registry`
/// (built-ins *and* the repo's custom rules) so a typo is refused at the CLI
/// rather than silently ignored later at check time.
pub fn mute_path(
    repo_root: &Path,
    registry: &crate::rules::Registry,
    path: &str,
    rule: Option<&str>,
    reason: Option<&str>,
    expires: Option<String>,
) -> Result<SuppressionRule, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("--path needs a glob, e.g. 'src/legacy/**'".to_string());
    }
    if let Some(selector) = rule {
        if !registry.known_selector(selector) {
            return Err(format!(
                "unknown rule '{selector}' (known: {})",
                registry.selector_names().join(", ")
            ));
        }
    }
    let rule = SuppressionRule {
        path: path.to_string(),
        rule: rule.map(str::to_string),
        hash: None,
        expires,
        reason: reason
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .unwrap_or(DEFAULT_MUTE_REASON)
            .to_string(),
    };
    append_mute(repo_root, &rule)?;
    Ok(rule)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suppress::last_check::{write_last_check, LastCheckHit};
    use std::path::PathBuf;

    const TODAY: &str = "2026-07-02";

    /// A scratch repo root with an `.argot/` dir inside it.
    fn scratch(name: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("argot_mute_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let argot_dir = root.join(".argot");
        std::fs::create_dir_all(&argot_dir).unwrap();
        (root, argot_dir)
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
        let (root, argot_dir) = scratch("append");
        seed_last_check(&argot_dir);
        let rule = mute_hash(
            &root,
            &argot_dir,
            crate::rules::Registry::builtin(),
            "abc123def456",
            None,
            None,
            TODAY,
        )
        .unwrap();
        assert_eq!(rule.path, "src/app.py");
        assert_eq!(rule.hash.as_deref(), Some("abc123def456"));
        assert_eq!(rule.reason, DEFAULT_MUTE_REASON);

        let loaded = ArgotConfig::load(&root).mutes(TODAY);
        assert_eq!(loaded.active, vec![rule]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn mute_preserves_existing_config_textually() {
        let (root, argot_dir) = scratch("preserve");
        seed_last_check(&argot_dir);
        std::fs::write(
            root.join("argot.toml"),
            "# hand-written\n[exclude]\npaths = [\"keep/\"]  # manual\n",
        )
        .unwrap();
        mute_hash(
            &root,
            &argot_dir,
            crate::rules::Registry::builtin(),
            "abc123def456",
            Some("noisy vendored hunk"),
            None,
            TODAY,
        )
        .unwrap();
        let content = std::fs::read_to_string(root.join("argot.toml")).unwrap();
        assert!(content.starts_with("# hand-written\n"), "comment preserved");
        assert!(content.contains("keep/"));
        assert!(content.contains("noisy vendored hunk"));
        let loaded = ArgotConfig::load(&root).mutes(TODAY);
        assert_eq!(loaded.active.len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn mute_unknown_hash_errors() {
        let (root, argot_dir) = scratch("unknown");
        seed_last_check(&argot_dir);
        let err = mute_hash(
            &root,
            &argot_dir,
            crate::rules::Registry::builtin(),
            "000000000000",
            None,
            None,
            TODAY,
        )
        .unwrap_err();
        assert!(err.contains("not found in the last check results"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn mute_without_last_check_errors() {
        let (root, argot_dir) = scratch("nocache");
        let err = mute_hash(
            &root,
            &argot_dir,
            crate::rules::Registry::builtin(),
            "abc123def456",
            None,
            None,
            TODAY,
        )
        .unwrap_err();
        assert!(err.contains("run `argot check` first"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn duplicate_mute_errors() {
        let (root, argot_dir) = scratch("dup");
        seed_last_check(&argot_dir);
        mute_hash(
            &root,
            &argot_dir,
            crate::rules::Registry::builtin(),
            "abc123def456",
            None,
            None,
            TODAY,
        )
        .unwrap();
        let err = mute_hash(
            &root,
            &argot_dir,
            crate::rules::Registry::builtin(),
            "abc123def456",
            None,
            None,
            TODAY,
        )
        .unwrap_err();
        assert!(err.contains("already muted"));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The path form is a standing rule, not a per-hit acceptance: it needs no
    /// prior check run, and an unknown rule selector is refused at write time
    /// rather than silently ignored on the next check.
    #[test]
    fn path_mutes_are_durable_and_validate_their_rule() {
        let (root, _) = scratch("path");
        let rule = mute_path(
            &root,
            crate::rules::Registry::builtin(),
            "src/legacy/**",
            Some("foreign-import"),
            Some("legacy tree, migrating in Q3"),
            None,
        )
        .unwrap();
        assert_eq!(rule.path, "src/legacy/**");
        assert_eq!(rule.rule.as_deref(), Some("foreign-import"));
        assert!(rule.hash.is_none(), "a path mute pins no hash");

        let loaded = ArgotConfig::load(&root).mutes(TODAY);
        assert_eq!(loaded.active, vec![rule]);

        let err = mute_path(
            &root,
            crate::rules::Registry::builtin(),
            "src/**",
            Some("no-such-rule"),
            None,
            None,
        )
        .unwrap_err();
        assert!(err.contains("unknown rule"), "{err}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn mute_with_expiry_records_date() {
        let (root, argot_dir) = scratch("expiry");
        seed_last_check(&argot_dir);
        let rule = mute_hash(
            &root,
            &argot_dir,
            crate::rules::Registry::builtin(),
            "abc123def456",
            None,
            Some("2026-08-01".to_string()),
            TODAY,
        )
        .unwrap();
        assert_eq!(rule.expires.as_deref(), Some("2026-08-01"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
