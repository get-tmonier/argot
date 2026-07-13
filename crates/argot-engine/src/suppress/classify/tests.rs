use super::*;

fn default_settings() -> crate::rules::RuleSettings {
    crate::rules::RuleSettings::resolve(&[])
}

fn mute(path: &str, rule: Option<&str>, hash: Option<&str>) -> SuppressionRule {
    SuppressionRule {
        path: path.to_string(),
        rule: rule.map(str::to_string),
        hash: hash.map(str::to_string),
        expires: None,
        reason: "test".to_string(),
    }
}

#[test]
fn exclude_beats_inline_beats_mute() {
    let settings = default_settings();
    let source = "# argot: ignore-next-line — noisy\nx = 1\ny = 2\n";
    let mutes = vec![mute("a.py", None, None)];
    // All three surfaces would match the line-2 finding; [exclude].paths wins.
    let s = FileSuppressions::parse(
        "a.py",
        source,
        Some("#"),
        &mutes,
        true,
        crate::rules::Registry::builtin(),
        &settings,
    );
    assert_eq!(
        s.classify("bpe", "abcdefabcdef", 2, 2),
        Some(SuppressedBy::Exclude)
    );
    // Without the exclude, the inline comment wins over the mute.
    let s = FileSuppressions::parse(
        "a.py",
        source,
        Some("#"),
        &mutes,
        false,
        crate::rules::Registry::builtin(),
        &settings,
    );
    assert_eq!(
        s.classify("bpe", "abcdefabcdef", 2, 2),
        Some(SuppressedBy::Inline)
    );
    // Outside the inline span, the mute catches it.
    assert_eq!(
        s.classify("bpe", "abcdefabcdef", 3, 3),
        Some(SuppressedBy::Mute)
    );
}

#[test]
fn unsuppressed_finding_classifies_none() {
    let settings = default_settings();
    let s = FileSuppressions::parse(
        "a.py",
        "x = 1\n",
        Some("#"),
        &[],
        false,
        crate::rules::Registry::builtin(),
        &settings,
    );
    assert_eq!(s.classify("bpe", "abcdefabcdef", 1, 1), None);
}

#[test]
fn no_comment_prefix_skips_inline_parsing() {
    let settings = default_settings();
    // Unknown language: the inline surface is inert, other surfaces still work.
    let source = "# argot: ignore-next-line — noisy\nx = 1\n";
    let s = FileSuppressions::parse(
        "a.unknown",
        source,
        None,
        &[],
        false,
        crate::rules::Registry::builtin(),
        &settings,
    );
    assert_eq!(s.classify("bpe", "abcdefabcdef", 2, 2), None);
    assert!(s.warnings().is_empty());
}

#[test]
fn malformed_inline_comment_surfaces_a_warning() {
    let settings = default_settings();
    // `ignore-next-line` without a reason is the canonical malformed comment.
    let source = "# argot: ignore-next-line\nx = 1\n";
    let s = FileSuppressions::parse(
        "a.py",
        source,
        Some("#"),
        &[],
        false,
        crate::rules::Registry::builtin(),
        &settings,
    );
    assert_eq!(s.warnings().len(), 1);
    assert_eq!(s.warnings()[0].line, 1);
    // The malformed comment suppresses nothing.
    assert_eq!(s.classify("bpe", "abcdefabcdef", 2, 2), None);
}

#[test]
fn mute_scoping_by_rule_and_hash_is_respected() {
    let settings = default_settings();
    let mutes = vec![mute("a.py", Some("rare-tokens"), Some("abcdefabcdef"))];
    let s = FileSuppressions::parse(
        "a.py",
        "x = 1\n",
        Some("#"),
        &mutes,
        false,
        crate::rules::Registry::builtin(),
        &settings,
    );
    // Same rule + hash → muted.
    assert_eq!(
        s.classify("bpe", "abcdefabcdef", 1, 1),
        Some(SuppressedBy::Mute)
    );
    // Different reason (rule scope mismatches) → reported.
    assert_eq!(s.classify("import", "abcdefabcdef", 1, 1), None);
    // Different hash → reported.
    assert_eq!(s.classify("bpe", "000000000000", 1, 1), None);
}
