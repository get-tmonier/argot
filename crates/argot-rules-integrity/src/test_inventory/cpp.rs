//! C++ test inventory: googletest `TEST`/`TEST_F`/`TEST_P`/`TYPED_TEST`/
//! `TYPED_TEST_P` macros and Catch2 `TEST_CASE`/`SCENARIO` macros.
//!
//! tree-sitter-cpp doesn't know these macros, so it parses their invocations
//! two different ways. `TEST(Suite, Name) { … }` parses as an ordinary
//! `function_definition`: the grammar can't tell "TEST" from a return type,
//! so it reads the declarator's name as "TEST" and — since `Suite`/`Name`
//! are bare identifiers, not typed parameters — each parameter_declaration
//! carries only a `type` field (no `declarator`). `TEST_CASE("name", tags)
//! { … }`, by contrast, has string-literal arguments the grammar can't read
//! as a parameter list at all, so it falls back to an `expression_statement`
//! (with a `MISSING ";"` recovery node) immediately followed, as a sibling,
//! by the body's `compound_statement` — the two are stitched back together
//! here by adjacency.
//!
//! Assertions: gtest `EXPECT_*`/`ASSERT_*`, Catch2 `REQUIRE*`/`CHECK*`, plus
//! `FAIL`/`ADD_FAILURE`. Skip markers: `GTEST_SKIP()`, Catch2 `SKIP(...)`.

use super::{ident_count, literals_and_key, node_text, walk_named, words_of};
use super::{AssertionSite, Strength, TestCase, TestInventory};

const LITERALS: &[&str] = &[
    "string_literal",
    "raw_string_literal",
    "number_literal",
    "char_literal",
    "true",
    "false",
    "null",
];

const GTEST_MACROS: &[&str] = &["TEST", "TEST_F", "TEST_P", "TYPED_TEST", "TYPED_TEST_P"];
const CATCH_MACROS: &[&str] = &["TEST_CASE", "SCENARIO"];

fn is_assertion(name: &str) -> bool {
    name.starts_with("EXPECT_")
        || name.starts_with("ASSERT_")
        || matches!(
            name,
            "REQUIRE"
                | "REQUIRE_FALSE"
                | "CHECK"
                | "CHECK_FALSE"
                | "REQUIRE_THAT"
                | "FAIL"
                | "ADD_FAILURE"
        )
}

fn strength_of(name: &str, arg_text: &str) -> Strength {
    match name {
        "EXPECT_EQ" | "ASSERT_EQ" | "EXPECT_STREQ" | "ASSERT_STREQ" | "EXPECT_STRCASEEQ"
        | "ASSERT_STRCASEEQ" | "EXPECT_FLOAT_EQ" | "ASSERT_FLOAT_EQ" | "EXPECT_DOUBLE_EQ"
        | "ASSERT_DOUBLE_EQ" => Strength::Exact,
        "EXPECT_NE" | "ASSERT_NE" | "EXPECT_STRNE" | "ASSERT_STRNE" | "EXPECT_STRCASENE"
        | "ASSERT_STRCASENE" | "EXPECT_LT" | "ASSERT_LT" | "EXPECT_LE" | "ASSERT_LE"
        | "EXPECT_GT" | "ASSERT_GT" | "EXPECT_GE" | "ASSERT_GE" | "EXPECT_NEAR" | "ASSERT_NEAR"
        | "REQUIRE_THAT" | "EXPECT_THAT" | "ASSERT_THAT" => Strength::Relational,
        "EXPECT_THROW" | "ASSERT_THROW" | "EXPECT_NO_THROW" | "ASSERT_NO_THROW"
        | "EXPECT_ANY_THROW" | "ASSERT_ANY_THROW" | "FAIL" | "ADD_FAILURE" => Strength::Existence,
        // EXPECT_TRUE/FALSE, ASSERT_TRUE/FALSE, REQUIRE, REQUIRE_FALSE, CHECK, CHECK_FALSE: the
        // macro alone doesn't say how much the argument pins down — read the argument text.
        _ => {
            if arg_text.contains("==") || arg_text.contains("!=") {
                Strength::Exact
            } else if arg_text.contains('<')
                || arg_text.contains('>')
                || arg_text.contains(".contains(")
            {
                Strength::Relational
            } else {
                Strength::Existence
            }
        }
    }
}

/// Walk a test body for assertion sites and skip markers, and build its
/// `TestCase`. Shared by both the gtest (`function_definition`) and Catch2
/// (adjacent `compound_statement`) shapes.
fn collect_test(
    name: String,
    line: usize,
    disabled: bool,
    body: tree_sitter::Node,
    src: &str,
) -> TestCase {
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
        if callee == "GTEST_SKIP" || callee == "SKIP" {
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
    TestCase {
        name,
        line,
        disabled,
        skip_markers,
        assertions,
        body_words: words_of(body_text),
        body_lines: body_text.lines().count(),
    }
}

pub(super) fn extract(root: tree_sitter::Node, src: &str) -> TestInventory {
    let mut inv = TestInventory::default();
    walk_named(root, &mut |n| match n.kind() {
        "function_definition" => {
            let Some(decl) = n.child_by_field_name("declarator") else {
                return;
            };
            if decl.kind() != "function_declarator" {
                return;
            }
            let Some(ident) = decl.child_by_field_name("declarator") else {
                return;
            };
            if ident.kind() != "identifier" {
                return;
            }
            let macro_name = node_text(ident, src);
            if !GTEST_MACROS.contains(&macro_name) {
                return;
            }
            let Some(params) = decl.child_by_field_name("parameters") else {
                return;
            };
            let args: Vec<&str> = (0..params.named_child_count())
                .filter_map(|i| params.named_child(i))
                .filter_map(|p| p.child_by_field_name("type"))
                .map(|t| node_text(t, src))
                .collect();
            let [suite, case] = args[..] else {
                return;
            };
            let Some(body) = n.child_by_field_name("body") else {
                return;
            };
            inv.tests.push(collect_test(
                format!("{suite}.{case}"),
                n.start_position().row + 1,
                suite.starts_with("DISABLED_") || case.starts_with("DISABLED_"),
                body,
                src,
            ));
        }
        "expression_statement" => {
            let Some(call) = n.named_child(0).filter(|c| c.kind() == "call_expression") else {
                return;
            };
            let Some(f) = call.child_by_field_name("function") else {
                return;
            };
            if f.kind() != "identifier" || !CATCH_MACROS.contains(&node_text(f, src)) {
                return;
            }
            let Some(name_node) = call
                .child_by_field_name("arguments")
                .and_then(|a| a.named_child(0))
                .filter(|a| a.kind() == "string_literal")
            else {
                return;
            };
            let name = node_text(name_node, src).trim_matches('"').to_string();
            let Some(body) = n
                .next_named_sibling()
                .filter(|s| s.kind() == "compound_statement")
            else {
                return;
            };
            inv.tests.push(collect_test(
                name,
                n.start_position().row + 1,
                false,
                body,
                src,
            ));
        }
        _ => {}
    });
    inv
}

#[cfg(test)]
mod tests {
    use crate::test_inventory::{extract, Strength};
    use argot_lang::adapters::Language;

    const SRC: &str = r#"
#include <gtest/gtest.h>

TEST(MathTest, AddsTwoNumbers) {
    int result = add(1, 2);
    EXPECT_EQ(result, 3);
    EXPECT_TRUE(x == 2);
    EXPECT_GT(result, 0);
}

TEST_F(FixtureTest, DISABLED_SlowThing) {
    GTEST_SKIP();
    EXPECT_TRUE(ok);
}

TEST_CASE("vectors can be sized", "[vector]") {
    REQUIRE(v.size() == 5);
}
"#;

    #[test]
    fn finds_gtest_and_catch2_tests_assertions_and_skips() {
        let inv = extract(SRC, Language::Cpp);
        assert_eq!(inv.tests.len(), 3);

        let math = &inv.tests[0];
        assert_eq!(math.name, "MathTest.AddsTwoNumbers");
        assert!(!math.disabled);
        assert_eq!(math.assertions.len(), 3);
        assert_eq!(math.assertions[0].callee, "EXPECT_EQ");
        assert_eq!(math.assertions[0].strength, Strength::Exact);
        assert_eq!(math.assertions[0].literals, ["3"]);
        assert_eq!(math.assertions[1].callee, "EXPECT_TRUE");
        assert_eq!(math.assertions[1].strength, Strength::Exact);
        assert_eq!(math.assertions[2].callee, "EXPECT_GT");
        assert_eq!(math.assertions[2].strength, Strength::Relational);

        let fixture = &inv.tests[1];
        assert_eq!(fixture.name, "FixtureTest.DISABLED_SlowThing");
        assert!(fixture.disabled);
        assert_eq!(fixture.skip_markers, 1);
        assert_eq!(fixture.assertions.len(), 1);
        assert_eq!(fixture.assertions[0].strength, Strength::Existence);

        let catch = &inv.tests[2];
        assert_eq!(catch.name, "vectors can be sized");
        assert_eq!(catch.assertions.len(), 1);
        assert_eq!(catch.assertions[0].callee, "REQUIRE");
        assert_eq!(catch.assertions[0].strength, Strength::Exact);
    }

    #[test]
    fn pure_literal_subject_is_a_tautology() {
        let inv = extract("TEST(T, X) { EXPECT_EQ(1, 1); }", Language::Cpp);
        assert_eq!(inv.tests[0].assertions[0].subject_idents, 0);
    }

    #[test]
    fn plain_functions_are_not_tests() {
        let inv = extract("int foo() { assert(x); return 1; }", Language::Cpp);
        assert!(inv.is_empty());
    }
}
