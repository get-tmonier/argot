//! PHP test inventory: PHPUnit `test*`-prefixed methods (case-insensitive)
//! or `#[Test]`-attributed methods inside a class, plus top-level
//! `test*`-prefixed functions; `$this->assert…`/`self::assert…`/
//! `static::assert…` and `expectException*` calls, and
//! `markTestSkipped`/`markTestIncomplete` calls plus a `#[Skip*]` attribute
//! as skip markers.

use super::{ident_count, literals_and_key, node_text, walk_named, words_of};
use super::{AssertionSite, Strength, TestCase, TestInventory};

const LITERALS: &[&str] = &[
    "string",
    "encapsed_string",
    "integer",
    "float",
    "boolean",
    "null",
];

const EXACT: &[&str] = &[
    "assertSame",
    "assertEquals",
    "assertCount",
    "assertJsonStringEqualsJsonString",
];
const RELATIONAL: &[&str] = &[
    "assertContains",
    "assertStringContainsString",
    "assertMatchesRegularExpression",
    "assertGreaterThan",
    "assertLessThan",
    "assertGreaterThanOrEqual",
    "assertLessThanOrEqual",
    "assertStringStartsWith",
    "assertStringEndsWith",
    "assertArrayHasKey",
    "assertInstanceOf",
];

/// Calls that mark the current test skipped/incomplete, wherever in the body
/// they occur (`Depends` is a dependency declaration, not a skip).
const SKIP_CALLS: &[&str] = &["markTestSkipped", "markTestIncomplete"];

fn strength_of(callee: &str) -> Strength {
    if EXACT.contains(&callee) {
        Strength::Exact
    } else if RELATIONAL.contains(&callee) {
        Strength::Relational
    } else {
        Strength::Existence
    }
}

/// PHPUnit's naming convention: `test`, case-insensitive on that prefix only
/// (`testFoo`, `test_foo`, `TestFoo`).
fn is_test_name(name: &str) -> bool {
    name.get(0..4)
        .is_some_and(|p| p.eq_ignore_ascii_case("test"))
}

/// Names of the attributes (`#[Test]`, `#[Skip]`, …) directly on `n`.
fn attribute_names(n: tree_sitter::Node, src: &str) -> Vec<String> {
    let mut names = Vec::new();
    let Some(attrs) = n.child_by_field_name("attributes") else {
        return names;
    };
    walk_named(attrs, &mut |c| {
        if c.kind() != "attribute" {
            return;
        }
        if let Some(name) = c.named_child(0) {
            names.push(node_text(name, src).to_string());
        }
    });
    names
}

/// Scan a test method/function body for skip markers
/// (`markTestSkipped`/`markTestIncomplete`) and `assert*`/`expectException*`
/// assertion sites (`$this->assert…`, `self::assert…`, `static::assert…`).
fn scan_body(
    n: tree_sitter::Node,
    src: &str,
    skip_markers: &mut usize,
    assertions: &mut Vec<AssertionSite>,
) {
    walk_named(n, &mut |c| {
        if c.kind() != "member_call_expression" && c.kind() != "scoped_call_expression" {
            return;
        }
        let Some(name_node) = c.child_by_field_name("name") else {
            return;
        };
        let name = node_text(name_node, src);

        if SKIP_CALLS.contains(&name) {
            *skip_markers += 1;
            return;
        }

        if !(name.starts_with("assert")
            || name.starts_with("expectException")
            || name == "expectNotToPerformAssertions")
        {
            return;
        }
        let (literals, site_key) = literals_and_key(c, src, LITERALS);
        let subject_idents = c
            .child_by_field_name("arguments")
            .map(ident_count)
            .unwrap_or(0);
        assertions.push(AssertionSite {
            callee: name.to_string(),
            site_key,
            literals,
            strength: strength_of(name),
            line: c.start_position().row + 1,
            subject_idents,
        });
    });
}

pub(super) fn extract(root: tree_sitter::Node, src: &str) -> TestInventory {
    let mut inv = TestInventory::default();
    walk_named(root, &mut |n| {
        if n.kind() != "method_declaration" && n.kind() != "function_definition" {
            return;
        }
        let Some(name_node) = n.child_by_field_name("name") else {
            return;
        };
        let name = node_text(name_node, src).to_string();
        let attrs = attribute_names(n, src);
        let has_test_attr = attrs.iter().any(|a| a == "Test");
        if !(is_test_name(&name) || has_test_attr) {
            return;
        }
        let mut skip_markers = attrs.iter().filter(|a| a.starts_with("Skip")).count();
        let mut assertions = Vec::new();
        let body = n.child_by_field_name("body").unwrap_or(n);
        scan_body(body, src, &mut skip_markers, &mut assertions);
        let body_text = node_text(body, src);
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

    const SRC: &str = r#"<?php

namespace App\Tests;

use PHPUnit\Framework\TestCase;
use PHPUnit\Framework\Attributes\Test;

class FooTest extends TestCase
{
    public function testAddition(): void
    {
        $result = add(1, 2);
        $this->assertSame(3, $result);
        $this->assertTrue($result > 0);
    }

    public function testFlaky(): void
    {
        $this->markTestSkipped('flaky on CI');
        $this->assertStringContainsString('a', 'abc');
    }

    #[Test]
    public function attributed(): void
    {
        static::assertEquals(1, 1);
        self::assertCount(2, [1, 2]);
    }

    public function helper(): int
    {
        return 3;
    }
}
"#;

    #[test]
    fn finds_test_methods_by_prefix_and_attribute() {
        let inv = extract(SRC, Language::Php);
        let names: Vec<&str> = inv.tests.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["testAddition", "testFlaky", "attributed"]);
    }

    #[test]
    fn assertions_carry_callees_strengths_and_literals() {
        let inv = extract(SRC, Language::Php);
        let addition = &inv.tests[0];
        assert_eq!(addition.assertions.len(), 2);
        assert_eq!(addition.assertions[0].callee, "assertSame");
        assert_eq!(addition.assertions[0].strength, Strength::Exact);
        assert_eq!(addition.assertions[0].literals, ["3"]);
        assert_eq!(addition.assertions[1].callee, "assertTrue");
        assert_eq!(addition.assertions[1].strength, Strength::Existence);
    }

    #[test]
    fn mark_test_skipped_is_a_skip_marker() {
        let inv = extract(SRC, Language::Php);
        let flaky = &inv.tests[1];
        assert_eq!(flaky.skip_markers, 1);
        assert_eq!(flaky.assertions.len(), 1);
        assert_eq!(flaky.assertions[0].callee, "assertStringContainsString");
        assert_eq!(flaky.assertions[0].strength, Strength::Relational);
    }

    #[test]
    fn scoped_calls_are_assertions_too() {
        let inv = extract(SRC, Language::Php);
        let attributed = &inv.tests[2];
        assert_eq!(attributed.assertions.len(), 2);
        assert_eq!(attributed.assertions[0].callee, "assertEquals");
        assert_eq!(attributed.assertions[0].strength, Strength::Exact);
        assert_eq!(attributed.assertions[1].callee, "assertCount");
    }

    #[test]
    fn non_test_methods_are_ignored() {
        let inv = extract(SRC, Language::Php);
        assert!(inv.tests.iter().all(|t| t.name != "helper"));
    }

    #[test]
    fn tautology_detection_via_subject_idents() {
        let tautology = extract(
            "<?php class T extends TestCase { public function testX() { $this->assertSame(1, 1); } }",
            Language::Php,
        );
        assert_eq!(tautology.tests[0].assertions[0].subject_idents, 0);

        let real = extract(
            "<?php class T extends TestCase { public function testX($x) { $this->assertSame(1, $x); } }",
            Language::Php,
        );
        assert!(real.tests[0].assertions[0].subject_idents > 0);
    }
}
