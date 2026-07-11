//! Java test inventory: JUnit4/5 `@Test`/`@ParameterizedTest`/`@RepeatedTest`/
//! `@TestFactory`/`@TestTemplate`-annotated methods; JUnit `assert*`/`fail`/
//! `verify` calls plus AssertJ `assertThat(…).matcher(…)` chains; and
//! `@Disabled`/`@Ignore` annotations plus `Assumptions.assumeTrue(…)` skip
//! markers.

use super::{ident_count, literals_and_key, node_text, walk_named, words_of};
use super::{AssertionSite, Strength, TestCase, TestInventory};

const LITERALS: &[&str] = &[
    "string_literal",
    "decimal_integer_literal",
    "hex_integer_literal",
    "octal_integer_literal",
    "binary_integer_literal",
    "decimal_floating_point_literal",
    "true",
    "false",
    "null_literal",
    "character_literal",
];

const TEST_ANNOTATIONS: &[&str] = &[
    "Test",
    "ParameterizedTest",
    "RepeatedTest",
    "TestFactory",
    "TestTemplate",
];

const DIRECT_EXACT: &[&str] = &[
    "assertEquals",
    "assertArrayEquals",
    "assertIterableEquals",
    "assertSame",
];
const DIRECT_RELATIONAL: &[&str] = &[
    "assertLinesMatch",
    "assertTimeout",
    "assertTimeoutPreemptively",
];

const ASSERTJ_EXACT: &[&str] = &["isEqualTo", "isSameAs", "hasSize", "containsExactly"];
const ASSERTJ_RELATIONAL: &[&str] = &[
    "contains",
    "containsAnyOf",
    "startsWith",
    "endsWith",
    "matches",
    "isGreaterThan",
    "isLessThan",
    "isBetween",
];

/// An annotation/marker_annotation's `name` field text, last segment only
/// (handles the rare fully-qualified `@org.junit.Test`).
fn annotation_name(n: tree_sitter::Node, src: &str) -> Option<String> {
    let name = n.child_by_field_name("name")?;
    let text = node_text(name, src);
    Some(text.rsplit('.').next().unwrap_or(text).to_string())
}

/// Walk a method_invocation's receiver chain (`object` field) looking for an
/// AssertJ `assertThat(…)` call — the subject of a fluent matcher chain.
fn assert_that_receiver<'t>(n: tree_sitter::Node<'t>, src: &str) -> Option<tree_sitter::Node<'t>> {
    let mut cur = n.child_by_field_name("object")?;
    loop {
        if cur.kind() != "method_invocation" {
            return None;
        }
        let name = cur.child_by_field_name("name")?;
        if node_text(name, src) == "assertThat" {
            return Some(cur);
        }
        cur = cur.child_by_field_name("object")?;
    }
}

pub(super) fn extract(root: tree_sitter::Node, src: &str) -> TestInventory {
    let mut inv = TestInventory::default();
    walk_named(root, &mut |n| {
        if n.kind() != "method_declaration" {
            return;
        }
        let mut modifiers = None;
        for i in 0..n.named_child_count() {
            if let Some(c) = n.named_child(i) {
                if c.kind() == "modifiers" {
                    modifiers = Some(c);
                    break;
                }
            }
        }
        let Some(modifiers) = modifiers else {
            return;
        };
        let mut annotation_names = Vec::new();
        for i in 0..modifiers.named_child_count() {
            let Some(a) = modifiers.named_child(i) else {
                continue;
            };
            if matches!(a.kind(), "annotation" | "marker_annotation") {
                if let Some(name) = annotation_name(a, src) {
                    annotation_names.push(name);
                }
            }
        }
        if !annotation_names
            .iter()
            .any(|a| TEST_ANNOTATIONS.contains(&a.as_str()))
        {
            return;
        }
        let mut skip_markers = annotation_names
            .iter()
            .filter(|a| a.as_str() == "Disabled" || a.as_str() == "Ignore")
            .count();
        let disabled = skip_markers > 0;
        let Some(name_node) = n.child_by_field_name("name") else {
            return;
        };
        let name = node_text(name_node, src).to_string();
        let mut assertions = Vec::new();
        walk_named(n, &mut |c| {
            if c.kind() != "method_invocation" {
                return;
            }
            let Some(name_field) = c.child_by_field_name("name") else {
                return;
            };
            let callee = node_text(name_field, src).to_string();
            let object_text = c
                .child_by_field_name("object")
                .map(|o| node_text(o, src))
                .unwrap_or("");
            if object_text == "Assumptions" && callee == "assumeTrue" {
                skip_markers += 1;
                return;
            }
            // Only the outermost call in a chain is the assertion site — an
            // inner call used as the `object` of a longer chain is skipped.
            if let Some(p) = c.parent() {
                if p.kind() == "method_invocation" && p.child_by_field_name("object") == Some(c) {
                    return;
                }
            }
            let is_direct = callee.starts_with("assert") || callee == "fail" || callee == "verify";
            if is_direct {
                let (literals, site_key) = literals_and_key(c, src, LITERALS);
                let strength = if DIRECT_EXACT.contains(&callee.as_str()) {
                    Strength::Exact
                } else if DIRECT_RELATIONAL.contains(&callee.as_str()) {
                    Strength::Relational
                } else {
                    Strength::Existence
                };
                let subject_idents = c
                    .child_by_field_name("arguments")
                    .map(ident_count)
                    .unwrap_or(0);
                assertions.push(AssertionSite {
                    callee,
                    site_key,
                    literals,
                    strength,
                    line: c.start_position().row + 1,
                    subject_idents,
                });
                return;
            }
            let Some(receiver) = assert_that_receiver(c, src) else {
                return;
            };
            let (literals, site_key) = literals_and_key(c, src, LITERALS);
            let strength = if ASSERTJ_EXACT.contains(&callee.as_str()) {
                Strength::Exact
            } else if ASSERTJ_RELATIONAL.contains(&callee.as_str()) {
                Strength::Relational
            } else {
                Strength::Existence
            };
            let subject_idents = c
                .child_by_field_name("arguments")
                .map(ident_count)
                .unwrap_or(0)
                + receiver
                    .child_by_field_name("arguments")
                    .map(ident_count)
                    .unwrap_or(0);
            assertions.push(AssertionSite {
                callee,
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
            disabled,
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
    use crate::scoring::adapters::Language;
    use crate::scoring::test_inventory::{extract, Strength};

    const SRC: &str = r#"
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Disabled;
import static org.junit.jupiter.api.Assertions.*;
import static org.assertj.core.api.Assertions.assertThat;

class CalculatorTest {
    @Test
    void addsTwoNumbers() {
        int result = Calculator.add(1, 2);
        assertEquals(3, result);
        assertTrue(result > 0);
        assertThat(result).isEqualTo(3);
    }

    @Disabled("flaky on CI")
    @Test
    void flakyTest() {
        Assumptions.assumeTrue(isCI());
        assertEquals(1, 1);
    }

    void helper() {
        return;
    }
}
"#;

    #[test]
    fn finds_tests_assertions_and_skips() {
        let inv = extract(SRC, Language::Java);
        let names: Vec<&str> = inv.tests.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["addsTwoNumbers", "flakyTest"]);

        let adds = &inv.tests[0];
        assert_eq!(adds.assertions.len(), 3);
        assert_eq!(adds.assertions[0].callee, "assertEquals");
        assert_eq!(adds.assertions[0].strength, Strength::Exact);
        assert_eq!(adds.assertions[0].literals, ["3"]);
        assert_eq!(adds.assertions[1].callee, "assertTrue");
        assert_eq!(adds.assertions[1].strength, Strength::Existence);
        assert_eq!(adds.assertions[2].callee, "isEqualTo");
        assert_eq!(adds.assertions[2].strength, Strength::Exact);
        assert_eq!(adds.assertions[2].literals, ["3"]);

        let flaky = &inv.tests[1];
        assert!(flaky.disabled);
        assert_eq!(flaky.skip_markers, 2);
        assert_eq!(flaky.assertions.len(), 1);
    }

    #[test]
    fn non_annotated_methods_are_not_tests() {
        let inv = extract(
            "class X { void helper() { assertEquals(1, 1); } }",
            Language::Java,
        );
        assert!(inv.is_empty());
    }

    #[test]
    fn pure_literal_subject_is_a_tautology() {
        let inv = extract(
            r#"class X {
                @Test
                void t() {
                    assertEquals(1, 1);
                    assertEquals(x.size(), 2);
                }
            }"#,
            Language::Java,
        );
        let t = &inv.tests[0];
        assert_eq!(t.assertions[0].subject_idents, 0);
        assert!(t.assertions[1].subject_idents > 0);
    }

    #[test]
    fn assertj_chain_only_counts_outermost_call() {
        let inv = extract(
            r#"class X {
                @Test
                void t() {
                    assertThat(name).startsWith("foo");
                }
            }"#,
            Language::Java,
        );
        let t = &inv.tests[0];
        assert_eq!(t.assertions.len(), 1);
        assert_eq!(t.assertions[0].callee, "startsWith");
        assert_eq!(t.assertions[0].strength, Strength::Relational);
        assert!(t.assertions[0].subject_idents > 0);
    }
}
