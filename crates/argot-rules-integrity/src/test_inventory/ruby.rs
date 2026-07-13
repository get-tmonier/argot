//! Ruby test inventory: minitest/test-unit `def test_*` methods, rails
//! minitest-spec `test "…" do … end`, and RSpec `it`/`specify` blocks (+
//! disabled `xit`/`xspecify`/`pending`-with-block); minitest `assert*` /
//! `refute*` calls and spec `must_*` / `wont_*` calls, RSpec
//! `expect(...).to`/`.not_to matcher(...)` chains, and bare
//! `skip`/`omit`/`pending` skip markers.

use super::{ident_count, literals_and_key, node_text, walk_named, words_of};
use super::{AssertionSite, Strength, TestCase, TestInventory};

const LITERALS: &[&str] = &[
    "string",
    "integer",
    "float",
    "true",
    "false",
    "nil",
    "simple_symbol",
    "symbol_array",
    "string_array",
    "regex",
];

const EXACT: &[&str] = &[
    "assert_equal",
    "refute_equal",
    "eq",
    "eql",
    "equal",
    "must_equal",
    "wont_equal",
    "assert_same",
];
const RELATIONAL: &[&str] = &[
    "assert_includes",
    "include",
    "assert_match",
    "match",
    "cover",
    "be_within",
    "must_include",
    "must_match",
    "assert_operator",
    "start_with",
    "end_with",
    "be",
];

/// Call heads that define a test case in block form (`head "name" do … end`).
const HEAD_TEST_NAMES: &[&str] = &["it", "specify", "xit", "xspecify", "pending", "test"];
/// Of those, the ones that mark the example disabled/pending.
const DISABLED_HEADS: &[&str] = &["xit", "xspecify", "pending"];

/// A string-literal node's text with its surrounding quotes stripped.
fn string_text(n: tree_sitter::Node, src: &str) -> String {
    node_text(n, src).trim_matches(['"', '\'']).to_string()
}

/// A call's subject identifiers, excluding the callee's own name: identifier
/// leaves in its `arguments`, plus (recursively) its receiver's — the chain
/// a matcher/assertion sits on, never the callee/matcher names themselves
/// (those live in the `method` field, which this never visits).
fn call_args_idents(n: tree_sitter::Node) -> usize {
    let mut total = n
        .child_by_field_name("arguments")
        .map(ident_count)
        .unwrap_or(0);
    // `expect { … }.to raise_error(X)` — the block IS the subject.
    if let Some(b) = n.child_by_field_name("block") {
        total += ident_count(b);
    }
    if let Some(r) = n.child_by_field_name("receiver") {
        total += if r.kind() == "call" {
            call_args_idents(r)
        } else {
            ident_count(r)
        };
    }
    total
}

/// The matcher's own callee name, walking a chained matcher
/// (`be_within(0.1).of(1.0)`) down to its root call; falls back to the
/// left-hand operand for an operator matcher (`be > 5`).
fn matcher_callee(mut n: tree_sitter::Node, src: &str) -> String {
    while n.kind() == "call" {
        match n.child_by_field_name("receiver") {
            Some(r) if r.kind() == "call" => n = r,
            _ => break,
        }
    }
    match n.kind() {
        "call" => n
            .child_by_field_name("method")
            .map(|m| node_text(m, src).to_string())
            .unwrap_or_default(),
        "binary" => n
            .child_by_field_name("left")
            .map(|l| node_text(l, src).to_string())
            .unwrap_or_default(),
        _ => node_text(n, src).trim().to_string(),
    }
}

fn strength_of(callee: &str) -> Strength {
    if EXACT.contains(&callee) {
        Strength::Exact
    } else if RELATIONAL.contains(&callee) {
        Strength::Relational
    } else {
        Strength::Existence
    }
}

/// Scan a test body for skip markers (`skip`/`omit`/bare `pending`) and
/// assertion sites (minitest `assert*`/`refute*`, minitest-spec
/// `must_*`/`wont_*`, RSpec `expect(...).to`/`.not_to matcher`).
fn scan_body(
    n: tree_sitter::Node,
    src: &str,
    skip_markers: &mut usize,
    assertions: &mut Vec<AssertionSite>,
) {
    walk_named(n, &mut |c| {
        if c.kind() != "call" {
            return;
        }
        let Some(method) = c.child_by_field_name("method") else {
            return;
        };
        let name = node_text(method, src);

        // A `pending("reason")` skip marker has no block; a `pending "…" do
        // … end` test case (handled by the outer walk) does — the block
        // check keeps the two from colliding.
        if matches!(name, "skip" | "omit" | "pending") && c.child_by_field_name("block").is_none() {
            *skip_markers += 1;
            return;
        }

        if name == "to" || name == "not_to" {
            let Some(receiver) = c.child_by_field_name("receiver") else {
                return;
            };
            let is_expect = receiver.kind() == "call"
                && receiver
                    .child_by_field_name("method")
                    .is_some_and(|m| node_text(m, src) == "expect");
            if !is_expect {
                return;
            }
            let Some(matcher) = c
                .child_by_field_name("arguments")
                .and_then(|a| a.named_child(0))
            else {
                return;
            };
            let callee = matcher_callee(matcher, src);
            let (literals, site_key) = literals_and_key(c, src, LITERALS);
            let subject_idents = call_args_idents(receiver) + call_args_idents(matcher);
            assertions.push(AssertionSite {
                strength: strength_of(&callee),
                callee,
                site_key,
                literals,
                line: c.start_position().row + 1,
                subject_idents,
            });
            return;
        }

        let is_minitest_assert = name.starts_with("assert") || name.starts_with("refute");
        let is_minitest_spec = name.starts_with("must_") || name.starts_with("wont_");
        if !(is_minitest_assert || is_minitest_spec) {
            return;
        }
        let (literals, site_key) = literals_and_key(c, src, LITERALS);
        assertions.push(AssertionSite {
            callee: name.to_string(),
            site_key,
            literals,
            strength: strength_of(name),
            line: c.start_position().row + 1,
            subject_idents: call_args_idents(c),
        });
    });
}

pub(super) fn extract(root: tree_sitter::Node, src: &str) -> TestInventory {
    let mut inv = TestInventory::default();
    walk_named(root, &mut |n| {
        // (a) minitest/test-unit: `def test_*`
        if n.kind() == "method" {
            let Some(name_node) = n.child_by_field_name("name") else {
                return;
            };
            let name = node_text(name_node, src).to_string();
            if !name.starts_with("test_") {
                return;
            }
            let mut skip_markers = 0usize;
            let mut assertions = Vec::new();
            scan_body(n, src, &mut skip_markers, &mut assertions);
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
            return;
        }

        // (b) rails minitest-spec `test "…" do … end` and (c) RSpec
        // `it`/`specify`/`xit`/`xspecify`/`pending` blocks.
        if n.kind() != "call" {
            return;
        }
        let Some(method) = n.child_by_field_name("method") else {
            return;
        };
        let head = node_text(method, src);
        if !HEAD_TEST_NAMES.contains(&head) {
            return;
        }
        let Some(block) = n.child_by_field_name("block") else {
            return;
        };
        let Some(name) = n
            .child_by_field_name("arguments")
            .and_then(|a| a.named_child(0))
            .filter(|a| a.kind() == "string")
            .map(|a| string_text(a, src))
        else {
            return;
        };
        let disabled = DISABLED_HEADS.contains(&head);
        let mut skip_markers = usize::from(disabled);
        let mut assertions = Vec::new();
        scan_body(block, src, &mut skip_markers, &mut assertions);
        let body_text = node_text(block, src);
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
    use crate::test_inventory::{extract, Strength};
    use argot_lang::adapters::Language;

    const SRC: &str = r#"
require 'test_helper'

class FooTest < Minitest::Test
  def test_addition
    result = add(1, 2)
    assert_equal 3, result
    refute_nil result
  end

  def test_flaky
    skip("flaky on CI")
    assert true
  end
end

RSpec.describe Foo do
  it "adds numbers" do
    x = add(1, 2)
    expect(x).to eq(3)
    expect(x).not_to be_nil
  end

  xit "legacy behaviour" do
    expect(legacy).to eq(true)
  end

  it "is a tautology" do
    expect(true).to eq(true)
  end
end
"#;

    #[test]
    fn finds_minitest_and_rspec_tests() {
        let inv = extract(SRC, Language::Ruby);
        let names: Vec<&str> = inv.tests.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "test_addition",
                "test_flaky",
                "adds numbers",
                "legacy behaviour",
                "is a tautology"
            ]
        );
    }

    #[test]
    fn minitest_assertions_carry_strength_and_literals() {
        let inv = extract(SRC, Language::Ruby);
        let add = &inv.tests[0];
        assert_eq!(add.assertions.len(), 2);
        assert_eq!(add.assertions[0].callee, "assert_equal");
        assert_eq!(add.assertions[0].strength, Strength::Exact);
        assert_eq!(add.assertions[0].literals, ["3"]);
        assert_eq!(add.assertions[1].callee, "refute_nil");
        assert_eq!(add.assertions[1].strength, Strength::Existence);
    }

    #[test]
    fn skip_call_is_a_skip_marker() {
        let inv = extract(SRC, Language::Ruby);
        let flaky = &inv.tests[1];
        assert_eq!(flaky.skip_markers, 1);
        assert_eq!(flaky.assertions.len(), 1);
        assert_eq!(flaky.assertions[0].callee, "assert");
    }

    #[test]
    fn rspec_expect_matchers_and_disabled_forms() {
        let inv = extract(SRC, Language::Ruby);
        let adds = &inv.tests[2];
        assert!(!adds.disabled);
        assert_eq!(adds.assertions.len(), 2);
        assert_eq!(adds.assertions[0].callee, "eq");
        assert_eq!(adds.assertions[0].strength, Strength::Exact);
        assert_eq!(adds.assertions[0].literals, ["3"]);
        assert_eq!(adds.assertions[1].callee, "be_nil");
        assert_eq!(adds.assertions[1].strength, Strength::Existence);

        let legacy = &inv.tests[3];
        assert!(legacy.disabled);
        assert_eq!(legacy.skip_markers, 1);
    }

    #[test]
    fn tautology_detection_via_subject_idents() {
        let inv = extract(SRC, Language::Ruby);
        let tautology = &inv.tests[4];
        assert_eq!(tautology.assertions[0].subject_idents, 0);

        let adds = &inv.tests[2];
        assert!(adds.assertions[0].subject_idents > 0);
    }
}
