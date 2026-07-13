//! TypeScript/JavaScript test inventory: `it/test("…", fn)` blocks (vitest /
//! jest / mocha / bun:test / node:test), `expect(…)` matcher chains plus
//! `assert.*` / `t.*` calls, and `it.skip` / `xit` / `.todo` disable forms.
//! The TS and JS grammars share the node kinds this walker touches.

use super::{ident_count, literals_and_key, node_text, walk_named, words_of};
use super::{AssertionSite, Strength, TestCase, TestInventory};

const LITERALS: &[&str] = &[
    "string",
    "number",
    "true",
    "false",
    "null",
    "undefined",
    "template_string",
    "regex",
];

const EXACT: &[&str] = &[
    "toBe",
    "toEqual",
    "toStrictEqual",
    "toMatchObject",
    "toMatchSnapshot",
    "toMatchInlineSnapshot",
    "toMatchFileSnapshot",
    "toHaveLength",
    "toHaveBeenCalledTimes",
    "deepEqual",
    "deepStrictEqual",
    "strictEqual",
    "equal",
    "equals",
    "eq",
    "is",
];
const RELATIONAL: &[&str] = &[
    "toContain",
    "toContainEqual",
    "toMatch",
    "toBeGreaterThan",
    "toBeGreaterThanOrEqual",
    "toBeLessThan",
    "toBeLessThanOrEqual",
    "toBeCloseTo",
    "toHaveProperty",
    "toBeInstanceOf",
    "includes",
    "match",
    "greaterThan",
    "lessThan",
    "closeTo",
    "contains",
];

/// Walk down a member/call chain to its root expression.
fn chain_root(mut n: tree_sitter::Node) -> tree_sitter::Node {
    loop {
        match n.kind() {
            "call_expression" => match n.child_by_field_name("function") {
                Some(f) => n = f,
                None => return n,
            },
            "member_expression" => match n.child_by_field_name("object") {
                Some(o) => n = o,
                None => return n,
            },
            "non_null_expression" | "parenthesized_expression" => match n.named_child(0) {
                Some(c) => n = c,
                None => return n,
            },
            _ => return n,
        }
    }
}

/// The innermost call in the chain whose function is a bare identifier
/// (`expect(subject)`) — its arguments are the assertion's subject.
fn subject_args(terminal: tree_sitter::Node) -> Option<tree_sitter::Node> {
    let mut n = terminal.child_by_field_name("function")?;
    loop {
        match n.kind() {
            "member_expression" => n = n.child_by_field_name("object")?,
            "non_null_expression" | "parenthesized_expression" => n = n.named_child(0)?,
            "call_expression" => {
                let f = n.child_by_field_name("function")?;
                if f.kind() == "identifier" {
                    return n.child_by_field_name("arguments");
                }
                n = f;
            }
            _ => return None,
        }
    }
}

pub(super) fn extract(root: tree_sitter::Node, src: &str) -> TestInventory {
    let mut inv = TestInventory::default();
    walk_named(root, &mut |n| {
        if n.kind() != "call_expression" {
            return;
        }
        let Some(f) = n.child_by_field_name("function") else {
            return;
        };
        let (head, prop) = match f.kind() {
            "identifier" => (node_text(f, src).to_string(), String::new()),
            "member_expression" => {
                let obj = f
                    .child_by_field_name("object")
                    .map(|o| node_text(o, src))
                    .unwrap_or("");
                let p = f
                    .child_by_field_name("property")
                    .map(|p| node_text(p, src))
                    .unwrap_or("");
                (obj.to_string(), p.to_string())
            }
            _ => return,
        };
        let is_test_head = matches!(head.as_str(), "it" | "test" | "xit" | "xtest");
        let is_test_call = is_test_head
            && (prop.is_empty()
                || matches!(
                    prop.as_str(),
                    "skip"
                        | "skipIf"
                        | "only"
                        | "todo"
                        | "fails"
                        | "each"
                        | "concurrent"
                        | "sequential"
                ));
        if !is_test_call {
            return;
        }
        let Some(args) = n.child_by_field_name("arguments") else {
            return;
        };
        let Some(name) = args
            .named_child(0)
            .filter(|a| a.kind() == "string" || a.kind() == "template_string")
            .map(|a| node_text(a, src).trim_matches(['"', '\'', '`']).to_string())
        else {
            return;
        };
        let disabled = head.starts_with('x') || prop == "skip" || prop == "todo";
        let mut assertions = Vec::new();
        walk_named(n, &mut |c| {
            if c.kind() != "call_expression" {
                return;
            }
            let Some(cf) = c.child_by_field_name("function") else {
                return;
            };
            if cf.kind() != "member_expression" {
                return;
            }
            let root_node = chain_root(c);
            let root_name =
                if root_node.kind() == "identifier" || root_node.kind() == "call_expression" {
                    node_text(root_node, src)
                } else {
                    return;
                };
            let matcher = cf
                .child_by_field_name("property")
                .map(|p| node_text(p, src))
                .unwrap_or("");
            let rooted_expect = root_name == "expect" || root_name.starts_with("expect(");
            let rooted_assert = root_name == "assert" || root_name == "t";
            if !(rooted_expect || rooted_assert) || matcher.is_empty() || matcher == "not" {
                return;
            }
            // Only terminal matcher calls: a longer member chain continues above.
            if let Some(p) = c.parent() {
                if p.kind() == "member_expression" {
                    return;
                }
            }
            let (literals, site_key) = literals_and_key(c, src, LITERALS);
            let strength = if EXACT.contains(&matcher) {
                Strength::Exact
            } else if RELATIONAL.contains(&matcher) {
                Strength::Relational
            } else {
                Strength::Existence
            };
            let mut subject_idents = c
                .child_by_field_name("arguments")
                .map(ident_count)
                .unwrap_or(0);
            if let Some(sub) = subject_args(c) {
                subject_idents += ident_count(sub);
            }
            assertions.push(AssertionSite {
                callee: matcher.to_string(),
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
            skip_markers: usize::from(disabled),
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
import { describe, it, expect } from 'vitest'

describe('router', () => {
  it('matches a static path', async () => {
    const res = await app.request('/hello')
    expect(res.status).toBe(200)
    expect(await res.text()).toContain('hello')
  })

  it.skip('handles unicode paths', () => {
    expect(match('/héllo')).toBeDefined()
  })

  xit('legacy behaviour', () => {
    expect(legacy()).toBe(true)
  })
})
"#;

    #[test]
    fn finds_tests_matchers_and_disabled_forms() {
        let inv = extract(SRC, Language::Typescript);
        assert_eq!(inv.tests.len(), 3);

        let first = &inv.tests[0];
        assert_eq!(first.name, "matches a static path");
        assert!(!first.disabled);
        assert_eq!(first.assertions.len(), 2);
        assert_eq!(first.assertions[0].callee, "toBe");
        assert_eq!(first.assertions[0].strength, Strength::Exact);
        assert_eq!(first.assertions[0].literals, ["200"]);
        assert_eq!(first.assertions[1].strength, Strength::Relational);

        assert!(inv.tests[1].disabled);
        assert!(inv.tests[2].disabled);
    }

    #[test]
    fn works_for_javascript_too() {
        let js = "test('adds', () => { expect(add(1, 2)).toEqual(3) })";
        let inv = extract(js, Language::Javascript);
        assert_eq!(inv.tests.len(), 1);
        assert_eq!(inv.tests[0].assertions.len(), 1);
        assert_eq!(inv.tests[0].assertions[0].literals.len(), 3);
    }

    #[test]
    fn tautology_detection_via_subject_idents() {
        let inv = extract(
            "it('x', () => { expect(true).toBe(true); expect(f(1)).toBe(true) })",
            Language::Typescript,
        );
        let t = &inv.tests[0];
        assert_eq!(t.assertions[0].subject_idents, 0);
        assert!(t.assertions[1].subject_idents > 0);
    }

    #[test]
    fn not_chain_still_reaches_the_terminal_matcher() {
        let inv = extract(
            "it('x', () => { expect(res.headers.get('etag')).not.toBeNull() })",
            Language::Typescript,
        );
        assert_eq!(inv.tests[0].assertions.len(), 1);
        assert_eq!(inv.tests[0].assertions[0].callee, "toBeNull");
    }
}
