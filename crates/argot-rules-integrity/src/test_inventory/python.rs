//! Python test inventory: pytest `test_*` functions + `unittest.TestCase`
//! methods; `assert` statements and `self.assert*`/`pytest.raises` calls;
//! `@pytest.mark.skip/skipif/xfail` and `@unittest.skip*` decorators.

use super::{ident_count, literals_and_key, node_text, walk_named, words_of};
use super::{AssertionSite, Strength, TestCase, TestInventory};

const LITERALS: &[&str] = &["string", "integer", "float", "true", "false", "none"];

pub(super) fn extract(root: tree_sitter::Node, src: &str) -> TestInventory {
    let mut inv = TestInventory::default();
    walk_named(root, &mut |n| {
        if n.kind() != "function_definition" {
            return;
        }
        let Some(name_node) = n.child_by_field_name("name") else {
            return;
        };
        let name = node_text(name_node, src).to_string();
        if !name.starts_with("test") {
            return;
        }
        let mut skip_markers = 0usize;
        // Decorators wrap the def in a decorated_definition; the test's span
        // must cover them (a tests-only skip decorator is test text, not
        // production text).
        let mut span_node = n;
        if let Some(parent) = n.parent() {
            if parent.kind() == "decorated_definition" {
                span_node = parent;
                for i in 0..parent.named_child_count() {
                    let Some(c) = parent.named_child(i) else {
                        continue;
                    };
                    if c.kind() == "decorator" {
                        let t = node_text(c, src);
                        if t.contains("skip") || t.contains("xfail") {
                            skip_markers += 1;
                        }
                    }
                }
            }
        }
        let mut assertions = Vec::new();
        walk_named(n, &mut |c| {
            if c.kind() == "assert_statement" {
                let (literals, site_key) = literals_and_key(c, src, LITERALS);
                let text = node_text(c, src);
                let strength = if text.contains("==") || text.contains("!=") {
                    Strength::Exact
                } else if text.contains(" in ")
                    || text.contains('<')
                    || text.contains('>')
                    || text.contains("startswith")
                    || text.contains("endswith")
                {
                    Strength::Relational
                } else {
                    Strength::Existence
                };
                assertions.push(AssertionSite {
                    callee: "assert".into(),
                    site_key,
                    literals,
                    strength,
                    line: c.start_position().row + 1,
                    subject_idents: ident_count(c),
                });
            } else if c.kind() == "call" {
                let Some(f) = c.child_by_field_name("function") else {
                    return;
                };
                let callee = node_text(f, src);
                let last = callee.rsplit('.').next().unwrap_or(callee);
                if !(last.starts_with("assert") || last == "raises") {
                    return;
                }
                let (literals, site_key) = literals_and_key(c, src, LITERALS);
                let strength = match last {
                    "assertEqual"
                    | "assertNotEqual"
                    | "assertDictEqual"
                    | "assertListEqual"
                    | "assertSequenceEqual"
                    | "assertTupleEqual"
                    | "assertMultiLineEqual"
                    | "assertCountEqual"
                    | "assertIs"
                    | "assertIsNot" => Strength::Exact,
                    "assertIn"
                    | "assertNotIn"
                    | "assertGreater"
                    | "assertLess"
                    | "assertGreaterEqual"
                    | "assertLessEqual"
                    | "assertRegex"
                    | "assertNotRegex"
                    | "assertAlmostEqual"
                    | "assertNotAlmostEqual" => Strength::Relational,
                    _ => Strength::Existence,
                };
                let subject_idents = c
                    .child_by_field_name("arguments")
                    .map(ident_count)
                    .unwrap_or(0);
                assertions.push(AssertionSite {
                    callee: last.to_string(),
                    site_key,
                    literals,
                    strength,
                    line: c.start_position().row + 1,
                    subject_idents,
                });
            }
        });
        let body_text = node_text(n, src);
        inv.tests.push(TestCase {
            name,
            line: span_node.start_position().row + 1,
            disabled: false,
            skip_markers,
            assertions,
            body_words: words_of(body_text),
            body_lines: node_text(span_node, src).lines().count(),
        });
    });
    inv
}

#[cfg(test)]
mod tests {
    use crate::test_inventory::{extract, Strength};
    use argot_lang::adapters::Language;

    const SRC: &str = r#"
import pytest

def helper():
    return 3

def test_addition():
    result = add(1, 2)
    assert result == 3
    assert result > 0

@pytest.mark.skip(reason="flaky on CI")
def test_flaky():
    assert fetch() is not None

class TestParser:
    def test_roundtrip(self):
        self.assertEqual(parse("x"), Node("x"))
        with pytest.raises(ValueError):
            parse("")
"#;

    #[test]
    fn finds_tests_assertions_and_skips() {
        let inv = extract(SRC, Language::Python);
        let names: Vec<&str> = inv.tests.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["test_addition", "test_flaky", "test_roundtrip"]);

        let add = &inv.tests[0];
        assert_eq!(add.assertions.len(), 2);
        assert_eq!(add.assertions[0].strength, Strength::Exact);
        assert_eq!(add.assertions[0].literals, ["3"]);
        assert_eq!(add.assertions[1].strength, Strength::Relational);

        let flaky = &inv.tests[1];
        assert_eq!(flaky.skip_markers, 1);

        let roundtrip = &inv.tests[2];
        assert_eq!(roundtrip.assertions.len(), 2);
        assert_eq!(roundtrip.assertions[0].callee, "assertEqual");
        assert_eq!(roundtrip.assertions[1].callee, "raises");
    }

    #[test]
    fn pure_literal_subject_is_a_tautology() {
        let inv = extract(
            "def test_x():\n    assert 1 == 1\n    assert x == 1\n",
            Language::Python,
        );
        let t = &inv.tests[0];
        assert_eq!(t.assertions[0].subject_idents, 0);
        assert!(t.assertions[1].subject_idents > 0);
    }

    #[test]
    fn site_key_blanks_literals_for_retarget_identity() {
        let a = extract("def test_x():\n    assert f(2) == 10\n", Language::Python);
        let b = extract("def test_x():\n    assert f(2) == 99\n", Language::Python);
        let (sa, sb) = (&a.tests[0].assertions[0], &b.tests[0].assertions[0]);
        assert_eq!(sa.site_key, sb.site_key);
        assert_ne!(sa.literals, sb.literals);
    }
}
