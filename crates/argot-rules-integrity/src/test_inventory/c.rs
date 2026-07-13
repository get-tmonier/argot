//! C test inventory: there's no universal C test framework, so this covers
//! the common shapes and degrades to an empty inventory otherwise — a
//! documented honest limit, not a bug. Test cases: any `function_definition`
//! whose name (found by walking the declarator chain, same as
//! [`super::defined_symbols`] — cmocka tests take a `void **state` pointer
//! parameter, so the name isn't directly on the outer declarator) starts
//! with `test_` or `Test`. Assertions: libc `assert`, cmocka `assert_*`,
//! Unity `TEST_ASSERT_*`, Check `ck_assert*`, CUnit `CU_ASSERT*`. Skip
//! markers: cmocka `skip()`, Unity `TEST_IGNORE()`.

use super::{ident_count, literals_and_key, node_text, walk_named, words_of};
use super::{AssertionSite, Strength, TestCase, TestInventory};

const LITERALS: &[&str] = &[
    "string_literal",
    "number_literal",
    "char_literal",
    "true",
    "false",
];

fn function_name<'a>(n: tree_sitter::Node, src: &'a str) -> Option<&'a str> {
    let mut d = n.child_by_field_name("declarator")?;
    loop {
        if d.kind() == "identifier" {
            return Some(node_text(d, src));
        }
        d = d.child_by_field_name("declarator")?;
    }
}

fn is_assertion(name: &str) -> bool {
    name == "assert"
        || name.starts_with("assert_")
        || name.starts_with("TEST_ASSERT")
        || name.starts_with("ck_assert")
        || name.starts_with("CU_ASSERT")
}

fn strength_of(name: &str, arg_text: &str) -> Strength {
    let lower = name.to_ascii_lowercase();
    if lower.contains("equal") || lower.contains("_eq") {
        Strength::Exact
    } else if lower.contains("range")
        || lower.contains("greater")
        || lower.contains("less")
        || lower.contains("in_set")
    {
        Strength::Relational
    } else if arg_text.contains("==") || arg_text.contains("!=") {
        Strength::Exact
    } else if arg_text.contains('<') || arg_text.contains('>') {
        Strength::Relational
    } else {
        Strength::Existence
    }
}

pub(super) fn extract(root: tree_sitter::Node, src: &str) -> TestInventory {
    let mut inv = TestInventory::default();
    walk_named(root, &mut |n| {
        if n.kind() != "function_definition" {
            return;
        }
        let Some(name) = function_name(n, src) else {
            return;
        };
        if !(name.starts_with("test_") || name.starts_with("Test")) {
            return;
        }
        let Some(body) = n.child_by_field_name("body") else {
            return;
        };
        let mut assertions = Vec::new();
        let mut skip_markers = 0usize;
        walk_named(body, &mut |c| {
            if c.kind() != "call_expression" {
                return;
            }
            let Some(f) = c.child_by_field_name("function") else {
                return;
            };
            if f.kind() != "identifier" {
                return;
            }
            let callee = node_text(f, src);
            if callee == "skip" || callee == "TEST_IGNORE" {
                skip_markers += 1;
                return;
            }
            if !is_assertion(callee) {
                return;
            }
            let args = c.child_by_field_name("arguments");
            let arg_text = args.map(|a| node_text(a, src)).unwrap_or("");
            let (literals, site_key) = literals_and_key(c, src, LITERALS);
            assertions.push(AssertionSite {
                callee: callee.to_string(),
                site_key,
                literals,
                strength: strength_of(callee, arg_text),
                line: c.start_position().row + 1,
                subject_idents: args.map(ident_count).unwrap_or(0),
            });
        });
        let body_text = node_text(body, src);
        inv.tests.push(TestCase {
            name: name.to_string(),
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
#include <cmocka.h>

static void test_parse(void **state) {
    (void) state;
    int result = parse("x");
    assert_int_equal(result, 1);
    assert_non_null(result);
}

void test_add(void) {
    assert(x == 1);
}

void helper(void) {
    assert(1 == 1);
}
"#;

    #[test]
    fn finds_cmocka_and_bare_assert_tests() {
        let inv = extract(SRC, Language::C);
        assert_eq!(inv.tests.len(), 2);

        let parse = &inv.tests[0];
        assert_eq!(parse.name, "test_parse");
        assert_eq!(parse.assertions.len(), 2);
        assert_eq!(parse.assertions[0].callee, "assert_int_equal");
        assert_eq!(parse.assertions[0].strength, Strength::Exact);
        assert_eq!(parse.assertions[1].callee, "assert_non_null");
        assert_eq!(parse.assertions[1].strength, Strength::Existence);

        let add = &inv.tests[1];
        assert_eq!(add.name, "test_add");
        assert_eq!(add.assertions.len(), 1);
        assert_eq!(add.assertions[0].callee, "assert");
        assert_eq!(add.assertions[0].strength, Strength::Exact);
    }

    #[test]
    fn pure_literal_subject_is_a_tautology() {
        let inv = extract("void test_x(void) { assert(1 == 1); }", Language::C);
        assert_eq!(inv.tests[0].assertions[0].subject_idents, 0);
    }

    #[test]
    fn non_test_function_with_asserts_is_not_a_test() {
        let inv = extract("void helper(void) { assert(x == 1); }", Language::C);
        assert!(inv.is_empty());
    }
}
