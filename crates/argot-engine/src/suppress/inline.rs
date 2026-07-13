//! Inline suppression comments — language-aware magic comments:
//!
//! ```text
//! # argot: ignore-next-line — <reason>
//! # argot: ignore-next-line rule=rare-tokens — <reason>
//! # argot: ignore-block-start — <reason>
//! …
//! # argot: ignore-block-end
//! ```
//!
//! (`//` for languages with C-style line comments; the adapter supplies the
//! prefix.) `rule=` accepts any rule or group name from the registry
//! ([`crate::rules`]). The separator before the reason may be `—`, `-`, or
//! `:`. A reason is mandatory: a suppression comment without one is reported
//! as a warning and ignored.

/// One inline suppression: an inclusive 1-indexed line range, an optional
/// rule/group scope, and the author's reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineRule {
    pub line_start: usize,
    pub line_end: usize,
    /// A rule or group name from the registry (`None` = every rule).
    pub rule: Option<String>,
    pub reason: String,
}

/// A malformed suppression comment (reported on stderr, directive ignored).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineWarning {
    /// 1-indexed line of the offending comment.
    pub line: usize,
    pub message: String,
}

/// All inline suppressions parsed from one file.
#[derive(Debug, Clone, Default)]
pub struct InlineSuppressions {
    pub rules: Vec<InlineRule>,
    pub warnings: Vec<InlineWarning>,
}

impl InlineSuppressions {
    /// Does any rule suppress a hit spanning `[line_start, line_end]` with the
    /// given winning reason code? Rule-scoped entries match when the hit's
    /// rule (or its group) equals the scope.
    pub fn suppresses(&self, line_start: usize, line_end: usize, reason_code: &str) -> bool {
        self.rules.iter().any(|r| {
            r.line_start <= line_end
                && line_start <= r.line_end
                && r.rule
                    .as_deref()
                    .is_none_or(|s| crate::rules::selector_matches_reason(s, reason_code))
        })
    }
}

/// Directive payload after the directive keyword: optional `rule=<name>`,
/// optional separator (`—`/`-`/`:`), then the reason.
struct Payload {
    rule: Option<String>,
    reason: String,
    error: Option<String>,
}

fn parse_payload(rest: &str, registry: &crate::rules::Registry) -> Payload {
    let mut rest = rest.trim();
    let mut rule = None;
    if let Some(after) = rest.strip_prefix("rule=") {
        let name: String = after.chars().take_while(|c| !c.is_whitespace()).collect();
        rest = after[name.len()..].trim_start();
        if !registry.known_selector(&name) {
            return Payload {
                rule: None,
                reason: String::new(),
                error: Some(format!(
                    "unknown rule '{name}' (expected one of: {})",
                    registry.selector_names().join(", ")
                )),
            };
        }
        rule = Some(name);
    }
    // Optional separator before the reason.
    for sep in ["—", "-", ":"] {
        if let Some(after) = rest.strip_prefix(sep) {
            rest = after;
            break;
        }
    }
    Payload {
        rule,
        reason: rest.trim().to_string(),
        error: None,
    }
}

/// Parse the inline suppression comments in `source`. `comment_prefix` is the
/// language's line-comment token (`#` for Python, `//` for TypeScript) —
/// supplied by the language adapter. This is a line-level scan: only comments
/// whose text starts with `argot:` are considered.
pub fn parse_inline(
    source: &str,
    comment_prefix: &str,
    registry: &crate::rules::Registry,
) -> InlineSuppressions {
    let mut out = InlineSuppressions::default();
    // Open block: (start_line, rule, reason).
    let mut open_block: Option<(usize, Option<String>, String)> = None;
    let mut last_line = 0usize;

    for (idx, line) in source.lines().enumerate() {
        let ln = idx + 1;
        last_line = ln;
        let Some(pos) = line.find(comment_prefix) else {
            continue;
        };
        let text = line[pos + comment_prefix.len()..].trim();
        let Some(directive) = text.strip_prefix("argot:") else {
            continue;
        };
        let directive = directive.trim_start();

        if let Some(rest) = directive.strip_prefix("ignore-next-line") {
            let p = parse_payload(rest, registry);
            if let Some(err) = p.error {
                out.warnings.push(InlineWarning {
                    line: ln,
                    message: err,
                });
                continue;
            }
            if p.reason.is_empty() {
                out.warnings.push(InlineWarning {
                    line: ln,
                    message: "suppression comment missing reason — ignored".to_string(),
                });
                continue;
            }
            out.rules.push(InlineRule {
                line_start: ln + 1,
                line_end: ln + 1,
                rule: p.rule,
                reason: p.reason,
            });
        } else if let Some(rest) = directive.strip_prefix("ignore-block-start") {
            let p = parse_payload(rest, registry);
            if let Some(err) = p.error {
                out.warnings.push(InlineWarning {
                    line: ln,
                    message: err,
                });
                continue;
            }
            if p.reason.is_empty() {
                out.warnings.push(InlineWarning {
                    line: ln,
                    message: "suppression comment missing reason — ignored".to_string(),
                });
                continue;
            }
            if open_block.is_some() {
                out.warnings.push(InlineWarning {
                    line: ln,
                    message: "nested ignore-block-start — previous block still open".to_string(),
                });
                continue;
            }
            open_block = Some((ln, p.rule, p.reason));
        } else if directive.starts_with("ignore-block-end") {
            match open_block.take() {
                Some((start, rule, reason)) => out.rules.push(InlineRule {
                    line_start: start,
                    line_end: ln,
                    rule,
                    reason,
                }),
                None => out.warnings.push(InlineWarning {
                    line: ln,
                    message: "ignore-block-end without a matching ignore-block-start".to_string(),
                }),
            }
        }
    }

    if let Some((start, rule, reason)) = open_block.take() {
        out.warnings.push(InlineWarning {
            line: start,
            message: "ignore-block-start never closed — suppressing to end of file".to_string(),
        });
        out.rules.push(InlineRule {
            line_start: start,
            line_end: last_line,
            rule,
            reason,
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_line_python() {
        let src = "x = 1\n# argot: ignore-next-line — vendored oddity\nweird()\nok()\n";
        let s = parse_inline(src, "#", crate::rules::Registry::builtin());
        assert_eq!(
            s.rules,
            vec![InlineRule {
                line_start: 3,
                line_end: 3,
                rule: None,
                reason: "vendored oddity".to_string(),
            }]
        );
        assert!(s.warnings.is_empty());
        assert!(s.suppresses(1, 5, "bpe"), "hunk spanning line 3 suppressed");
        assert!(
            !s.suppresses(4, 5, "bpe"),
            "hunk below line 3 not suppressed"
        );
    }

    #[test]
    fn next_line_typescript() {
        let src = "const a = 1;\n// argot: ignore-next-line - generated glue\nweird();\n";
        let s = parse_inline(src, "//", crate::rules::Registry::builtin());
        assert_eq!(s.rules.len(), 1);
        assert_eq!(s.rules[0].line_start, 3);
        assert_eq!(s.rules[0].reason, "generated glue");
    }

    #[test]
    fn block_suppression() {
        let src = "\
a()
# argot: ignore-block-start — legacy shim
b()
c()
# argot: ignore-block-end
d()
";
        let s = parse_inline(src, "#", crate::rules::Registry::builtin());
        assert_eq!(s.rules.len(), 1);
        assert_eq!((s.rules[0].line_start, s.rules[0].line_end), (2, 5));
        assert!(s.suppresses(3, 4, "import"));
        assert!(!s.suppresses(6, 6, "import"));
    }

    #[test]
    fn rule_scoped_entry_only_matches_its_rule() {
        let src = "# argot: ignore-next-line rule=rare-tokens — noisy tokens\nweird()\n";
        let s = parse_inline(src, "#", crate::rules::Registry::builtin());
        assert_eq!(s.rules[0].rule.as_deref(), Some("rare-tokens"));
        assert!(s.suppresses(2, 2, "bpe"));
        assert!(!s.suppresses(2, 2, "import"));
        assert!(!s.suppresses(2, 2, "call_receiver"));
    }

    #[test]
    fn group_scoped_entry_matches_every_rule_in_the_group() {
        let src = "# argot: ignore-next-line rule=semantic — intentional twin\nweird()\n";
        let s = parse_inline(src, "#", crate::rules::Registry::builtin());
        assert!(s.suppresses(2, 2, "redundant"));
        assert!(s.suppresses(2, 2, "misplaced"));
        assert!(!s.suppresses(2, 2, "bpe"));
    }

    #[test]
    fn semantic_rules_are_suppressible_by_name() {
        let src =
            "# argot: ignore-next-line rule=redundant — intentional reimplementation\nweird()\n";
        let s = parse_inline(src, "#", crate::rules::Registry::builtin());
        assert!(s.suppresses(2, 2, "redundant"));
        assert!(!s.suppresses(2, 2, "misplaced"));
    }

    #[test]
    fn separator_variants_accepted() {
        for sep in ["—", "-", ":"] {
            let src = format!("# argot: ignore-next-line {sep} why\nx()\n");
            let s = parse_inline(&src, "#", crate::rules::Registry::builtin());
            assert_eq!(s.rules[0].reason, "why", "separator {sep:?}");
        }
    }

    #[test]
    fn missing_reason_warns_and_ignores() {
        let src = "# argot: ignore-next-line\nweird()\n# argot: ignore-block-start\nx()\n";
        let s = parse_inline(src, "#", crate::rules::Registry::builtin());
        assert!(s.rules.is_empty());
        assert_eq!(s.warnings.len(), 2);
        assert!(s.warnings[0].message.contains("missing reason"));
        assert_eq!(s.warnings[0].line, 1);
        assert_eq!(s.warnings[1].line, 3);
        assert!(!s.suppresses(2, 2, "bpe"));
    }

    #[test]
    fn unknown_rule_warns_and_ignores() {
        let src = "# argot: ignore-next-line rule=quantum — hmm\nweird()\n";
        let s = parse_inline(src, "#", crate::rules::Registry::builtin());
        assert!(s.rules.is_empty());
        assert_eq!(s.warnings.len(), 1);
        assert!(s.warnings[0].message.contains("unknown rule 'quantum'"));
    }

    #[test]
    fn unclosed_block_suppresses_to_eof_with_warning() {
        let src = "a()\n# argot: ignore-block-start — tail is generated\nb()\nc()\n";
        let s = parse_inline(src, "#", crate::rules::Registry::builtin());
        assert_eq!(s.rules.len(), 1);
        assert_eq!((s.rules[0].line_start, s.rules[0].line_end), (2, 4));
        assert_eq!(s.warnings.len(), 1);
        assert!(s.warnings[0].message.contains("never closed"));
    }

    #[test]
    fn stray_block_end_warns() {
        let src = "# argot: ignore-block-end\n";
        let s = parse_inline(src, "#", crate::rules::Registry::builtin());
        assert!(s.rules.is_empty());
        assert!(s.warnings[0].message.contains("without a matching"));
    }

    #[test]
    fn non_argot_comments_are_ignored() {
        let src = "# just a comment\nx = 1  # argot: not-a-directive\n";
        let s = parse_inline(src, "#", crate::rules::Registry::builtin());
        assert!(s.rules.is_empty());
        assert!(s.warnings.is_empty());
    }
}
