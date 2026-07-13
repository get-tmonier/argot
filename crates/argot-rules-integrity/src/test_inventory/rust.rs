//! Rust test inventory: `#[test]`-attributed functions (incl. `#[tokio::test]`
//! and friends), `assert!`/`assert_eq!`/`assert_ne!`/`panic!` macro sites, and
//! `#[ignore]` markers. Unit tests live inside production files, so the
//! detector treats test identity per function, never per file.

use super::{ident_count, literals_and_key, node_text, walk_named, words_of};
use super::{AssertionSite, Strength, TestCase, TestInventory};

const LITERALS: &[&str] = &[
    "string_literal",
    "raw_string_literal",
    "integer_literal",
    "float_literal",
    "boolean_literal",
    "char_literal",
];

pub(super) fn extract(root: tree_sitter::Node, src: &str) -> TestInventory {
    let mut inv = TestInventory::default();
    walk_named(root, &mut |n| {
        if n.kind() != "function_item" {
            return;
        }
        // Attributes are preceding named siblings of the item.
        let mut is_test = false;
        let mut ignored = false;
        // The span starts at the first attribute: markers are siblings, and
        // the production-side strip must blank them with the test (a
        // tests-only `#[ignore]` addition must not read as a prod change).
        let mut start_line = n.start_position().row + 1;
        let mut sib = n.prev_named_sibling();
        while let Some(s) = sib {
            if s.kind() != "attribute_item" {
                break;
            }
            let t = node_text(s, src);
            // #[test], #[tokio::test], #[rstest], #[test_case(…)] …
            if t.contains("test") {
                is_test = true;
            }
            if t.contains("ignore") {
                ignored = true;
            }
            start_line = s.start_position().row + 1;
            sib = s.prev_named_sibling();
        }
        if !is_test {
            return;
        }
        let name = n
            .child_by_field_name("name")
            .map(|x| node_text(x, src).to_string())
            .unwrap_or_default();
        let mut assertions = Vec::new();
        walk_named(n, &mut |c| {
            if c.kind() != "macro_invocation" {
                return;
            }
            let Some(mac) = c.child_by_field_name("macro") else {
                return;
            };
            let mname = node_text(mac, src);
            let base = mname.rsplit("::").next().unwrap_or(mname);
            if !(base.starts_with("assert") || base == "panic" || base.starts_with("debug_assert"))
            {
                return;
            }
            let (literals, site_key) = literals_and_key(c, src, LITERALS);
            let strength = if base.ends_with("_eq") || base.ends_with("_ne") {
                Strength::Exact
            } else {
                let t = node_text(c, src);
                if t.contains("==") || t.contains("!=") {
                    Strength::Exact
                } else if t.contains('<') || t.contains('>') || t.contains(".contains(") {
                    Strength::Relational
                } else {
                    Strength::Existence
                }
            };
            let mut subject_idents = 0usize;
            for i in 0..c.named_child_count() {
                if let Some(ch) = c.named_child(i) {
                    if ch.kind() == "token_tree" {
                        subject_idents += ident_count(ch);
                    }
                }
            }
            assertions.push(AssertionSite {
                callee: base.to_string(),
                site_key,
                literals,
                strength,
                line: c.start_position().row + 1,
                subject_idents,
            });
        });
        let body_text = node_text(n, src);
        let end_line = n.end_position().row + 1;
        inv.tests.push(TestCase {
            name,
            line: start_line,
            disabled: ignored,
            skip_markers: usize::from(ignored),
            assertions,
            body_words: words_of(body_text),
            body_lines: end_line + 1 - start_line,
        });
    });
    inv
}

#[cfg(test)]
mod tests {
    use crate::test_inventory::{extract, Strength};
    use argot_lang::adapters::Language;

    const SRC: &str = r#"
pub fn shell_escape(s: &str) -> String { s.to_string() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        assert_eq!(shell_escape(""), "''");
        assert!(shell_escape("x").len() > 0);
    }

    #[test]
    #[ignore]
    fn slow_roundtrip() {
        assert_eq!(roundtrip(BIG), BIG);
    }
}
"#;

    #[test]
    fn finds_in_file_unit_tests() {
        let inv = extract(SRC, Language::Rust);
        assert_eq!(inv.tests.len(), 2);
        let empty = &inv.tests[0];
        assert_eq!(empty.name, "empty");
        assert_eq!(empty.assertions.len(), 2);
        assert_eq!(empty.assertions[0].strength, Strength::Exact);
        assert_eq!(empty.assertions[1].strength, Strength::Relational);
        assert!(inv.tests[1].disabled);
        assert_eq!(inv.tests[1].skip_markers, 1);
    }

    #[test]
    fn plain_functions_are_not_tests() {
        let inv = extract("fn helper() { assert!(true); }", Language::Rust);
        assert!(inv.is_empty());
    }
}
