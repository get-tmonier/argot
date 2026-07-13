use super::*;

const LOCKED_TOML: &str = r#"
[rules]
misplaced = "warn"
"foreign-import" = { severity = "error", locked = true }
integrity = { severity = "error", locked = true }

[[mute]]
path = "src/**"
rule = "misplaced"
reason = "legacy"
"#;

#[test]
fn lock_state_reads_locks_and_scoped_mutes() {
    let state = lock_state(LOCKED_TOML);
    assert_eq!(
        state.locked.get("foreign-import").map(String::as_str),
        Some("error")
    );
    assert_eq!(
        state.locked.get("integrity").map(String::as_str),
        Some("error")
    );
    assert!(
        !state.locked.contains_key("misplaced"),
        "plain severity is not a lock"
    );
    assert_eq!(state.mutes.get("misplaced"), Some(&1));
    // Malformed TOML degrades to no locks, never a panic.
    assert!(lock_state("[rules\nnot toml").locked.is_empty());
}

#[test]
fn weakened_orders_the_severity_triad() {
    assert!(weakened("error", "warn"));
    assert!(weakened("error", "off"));
    assert!(weakened("warn", "off"));
    assert!(!weakened("warn", "error"));
    assert!(!weakened("error", "error"));
    // Unparseable new severities read as strict — never a false tamper.
    assert!(!weakened("error", "loud"));
}

#[test]
fn group_locks_cover_their_rules_mutes() {
    assert!(lock_covers("integrity", "test-deleted"));
    assert!(lock_covers("foreign-import", "foreign-import"));
    assert!(!lock_covers("integrity", "foreign-import"));
    assert!(!lock_covers("voice", "test-deleted"));
}
