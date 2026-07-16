//! Go test inventory: `func TestXxx(t *testing.T)` (+ Benchmark/Fuzz),
//! `t.Error*/t.Fatal*/t.Fail*` plus testify `assert.*`/`require.*` calls,
//! and `t.Skip*` markers.

use super::{ident_count, literals_and_key, node_text, walk_named, words_of};
use super::{AssertionSite, Strength, TestCase, TestInventory};

const LITERALS: &[&str] = &[
    "interpreted_string_literal",
    "raw_string_literal",
    "int_literal",
    "float_literal",
    "true",
    "false",
    "nil",
    "rune_literal",
];

pub(super) fn extract(root: tree_sitter::Node, src: &str) -> TestInventory {
    let mut inv = TestInventory::default();
    walk_named(root, &mut |n| {
        if n.kind() != "function_declaration" {
            return;
        }
        let Some(name_node) = n.child_by_field_name("name") else {
            return;
        };
        let name = node_text(name_node, src).to_string();
        if !(name.starts_with("Test") || name.starts_with("Benchmark") || name.starts_with("Fuzz"))
        {
            return;
        }
        let params = n
            .child_by_field_name("parameters")
            .map(|p| node_text(p, src))
            .unwrap_or("");
        if !(params.contains("testing.T")
            || params.contains("testing.B")
            || params.contains("testing.F"))
        {
            return;
        }
        let mut assertions = Vec::new();
        let mut skip_markers = 0usize;
        walk_named(n, &mut |c| {
            if c.kind() != "call_expression" {
                return;
            }
            let Some(f) = c.child_by_field_name("function") else {
                return;
            };
            if f.kind() != "selector_expression" {
                return;
            }
            let sel = node_text(f, src);
            let field = f
                .child_by_field_name("field")
                .map(|x| node_text(x, src))
                .unwrap_or("");
            let is_t = sel.starts_with("t.") || sel.starts_with("b.");
            if is_t && field.starts_with("Skip") {
                skip_markers += 1;
                return;
            }
            let is_assert = (is_t
                && (field.starts_with("Error")
                    || field.starts_with("Fatal")
                    || field == "Fail"
                    || field == "FailNow"))
                || sel.starts_with("assert.")
                || sel.starts_with("require.");
            if !is_assert {
                return;
            }
            let (literals, site_key) = literals_and_key(c, src, LITERALS);
            let strength = if field.contains("Equal") || field == "Exactly" || field == "Same" {
                Strength::Exact
            } else if field.contains("Contains")
                || field.contains("Greater")
                || field.contains("Less")
                || field.contains("Regexp")
                || field.contains("InDelta")
            {
                Strength::Relational
            } else {
                Strength::Existence
            };
            let subject_idents = c
                .child_by_field_name("arguments")
                .map(|a| {
                    let mut count = ident_count(a);
                    // Discount the testing handles passed as plain args.
                    for ch in argot_lang::ts_parse::named_child_nodes(a) {
                        if ch.kind() == "identifier" {
                            let t = node_text(ch, src);
                            if t == "t" || t == "b" {
                                count = count.saturating_sub(1);
                            }
                        }
                    }
                    count
                })
                .unwrap_or(0);
            assertions.push(AssertionSite {
                callee: format!("{}.{}", sel.split('.').next().unwrap_or(""), field),
                site_key,
                literals,
                strength,
                line: c.start_position().row + 1,
                subject_idents,
            });
        });
        let body_text = node_text(n, src);
        inv.tests.push(TestCase {
            name,
            line: n.start_position().row + 1,
            disabled: false,
            skip_markers,
            assertions,
            body_words: words_of(body_text),
            body_lines: body_text.lines().count(),
        });
    });
    inv
}

#[cfg(test)]
mod tests {
    use crate::test_inventory::{extract, Strength};
    use argot_lang::adapters::Language;

    const SRC: &str = r#"
package api

import "testing"

func helper() int { return 3 }

func TestGraphQL(t *testing.T) {
    resp, err := client.GraphQL("query {}")
    if err != nil {
        t.Fatalf("unexpected error: %v", err)
    }
    assert.Equal(t, "expected", resp.Data)
    require.NoError(t, err)
}

func TestFlaky(t *testing.T) {
    t.Skip("flaky on windows")
    assert.Contains(t, out, "warning")
}
"#;

    #[test]
    fn finds_tests_assertions_and_skips() {
        let inv = extract(SRC, Language::Go);
        assert_eq!(inv.tests.len(), 2);

        let gql = &inv.tests[0];
        assert_eq!(gql.name, "TestGraphQL");
        assert_eq!(gql.assertions.len(), 3);
        assert_eq!(gql.assertions[1].callee, "assert.Equal");
        assert_eq!(gql.assertions[1].strength, Strength::Exact);
        assert_eq!(gql.assertions[2].strength, Strength::Existence);

        let flaky = &inv.tests[1];
        assert_eq!(flaky.skip_markers, 1);
        assert_eq!(flaky.assertions[0].strength, Strength::Relational);
    }

    #[test]
    fn non_test_funcs_are_ignored() {
        let inv = extract("package x\nfunc Process(t int) {}\n", Language::Go);
        assert!(inv.is_empty());
    }
}
