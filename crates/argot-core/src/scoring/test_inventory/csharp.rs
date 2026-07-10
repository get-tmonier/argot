//! C# test inventory: xUnit `[Fact]`/`[Theory]`, NUnit `[Test]`/`[TestCase]`,
//! MSTest `[TestMethod]`/`[DataTestMethod]`-attributed methods; `Assert.*`/
//! `CollectionAssert.*`/`StringAssert.*` calls plus FluentAssertions
//! `subject.Should().Matcher(…)` chains; and `[Ignore]`/`[Fact(Skip = …)]`
//! attributes plus `Assert.Ignore(…)`/`Assert.Inconclusive(…)` skip markers.

use super::{ident_count, literals_and_key, node_text, walk_named, words_of};
use super::{AssertionSite, Strength, TestCase, TestInventory};

const LITERALS: &[&str] = &[
    "string_literal",
    "verbatim_string_literal",
    "raw_string_literal",
    "integer_literal",
    "real_literal",
    "boolean_literal",
    "null_literal",
    "character_literal",
];

const TEST_ATTRIBUTES: &[&str] = &[
    "Fact",
    "Theory",
    "Test",
    "TestMethod",
    "TestCase",
    "DataTestMethod",
];

const EXACT: &[&str] = &[
    "AreEqual",
    "AreSame",
    "Equal",
    "Same",
    "BeEquivalentTo",
    "Be",
    "HaveCount",
    "SequenceEqual",
];
const RELATIONAL: &[&str] = &[
    "Contains",
    "Contain",
    "StringContaining",
    "Match",
    "BeGreaterThan",
    "BeLessThan",
    "InRange",
    "StartWith",
    "EndWith",
    "GreaterThan",
    "Less",
];

fn strength_of(callee: &str) -> Strength {
    if EXACT.contains(&callee) {
        Strength::Exact
    } else if RELATIONAL.contains(&callee) {
        Strength::Relational
    } else {
        Strength::Existence
    }
}

/// A method's attributes: every `attribute` node across its (possibly
/// several, one per `[…]` bracket group) `attribute_list` siblings, paired
/// with its name text.
fn method_attributes<'t>(
    n: tree_sitter::Node<'t>,
    src: &str,
) -> Vec<(String, tree_sitter::Node<'t>)> {
    let mut out = Vec::new();
    for i in 0..n.named_child_count() {
        let Some(list) = n.named_child(i) else {
            continue;
        };
        if list.kind() != "attribute_list" {
            continue;
        }
        for j in 0..list.named_child_count() {
            let Some(a) = list.named_child(j) else {
                continue;
            };
            if a.kind() != "attribute" {
                continue;
            }
            if let Some(name) = a.child_by_field_name("name") {
                out.push((node_text(name, src).to_string(), a));
            }
        }
    }
    out
}

/// `subject.Should().Matcher(…)` — the terminal call's receiver chains
/// directly off a `.Should()` call. Returns the root expression (`subject`)
/// the chain hangs off.
fn should_chain_root<'t>(n: tree_sitter::Node<'t>, src: &str) -> Option<tree_sitter::Node<'t>> {
    let func = n.child_by_field_name("function")?;
    if func.kind() != "member_access_expression" {
        return None;
    }
    let receiver = func.child_by_field_name("expression")?;
    if receiver.kind() != "invocation_expression" {
        return None;
    }
    let inner_func = receiver.child_by_field_name("function")?;
    if inner_func.kind() != "member_access_expression" {
        return None;
    }
    let inner_name = inner_func.child_by_field_name("name")?;
    if node_text(inner_name, src) != "Should" {
        return None;
    }
    inner_func.child_by_field_name("expression")
}

pub(super) fn extract(root: tree_sitter::Node, src: &str) -> TestInventory {
    let mut inv = TestInventory::default();
    walk_named(root, &mut |n| {
        if n.kind() != "method_declaration" {
            return;
        }
        let attrs = method_attributes(n, src);
        if !attrs
            .iter()
            .any(|(name, _)| TEST_ATTRIBUTES.contains(&name.as_str()))
        {
            return;
        }
        let mut skip_markers = 0usize;
        let mut disabled = false;
        for (name, node) in &attrs {
            if name == "Ignore" {
                skip_markers += 1;
                disabled = true;
                continue;
            }
            if !TEST_ATTRIBUTES.contains(&name.as_str()) {
                continue;
            }
            for k in 0..node.named_child_count() {
                let Some(al) = node.named_child(k) else {
                    continue;
                };
                if al.kind() != "attribute_argument_list" {
                    continue;
                }
                for m in 0..al.named_child_count() {
                    let Some(arg) = al.named_child(m) else {
                        continue;
                    };
                    if node_text(arg, src).contains("Skip") {
                        skip_markers += 1;
                        disabled = true;
                    }
                }
            }
        }
        let Some(name_node) = n.child_by_field_name("name") else {
            return;
        };
        let name = node_text(name_node, src).to_string();
        let mut assertions = Vec::new();
        walk_named(n, &mut |c| {
            if c.kind() != "invocation_expression" {
                return;
            }
            let Some(func) = c.child_by_field_name("function") else {
                return;
            };
            if func.kind() != "member_access_expression" {
                return;
            }
            // Only the outermost call in a chain is the assertion site — an
            // inner call embedded as the receiver of a further member access
            // is skipped.
            if let Some(p) = c.parent() {
                if p.kind() == "member_access_expression" {
                    return;
                }
            }
            let Some(callee_node) = func.child_by_field_name("name") else {
                return;
            };
            let callee = node_text(callee_node, src).to_string();
            let receiver_text = func
                .child_by_field_name("expression")
                .map(|r| node_text(r, src))
                .unwrap_or("");
            if receiver_text == "Assert" && matches!(callee.as_str(), "Ignore" | "Inconclusive") {
                skip_markers += 1;
                return;
            }
            let is_direct = matches!(
                receiver_text,
                "Assert" | "CollectionAssert" | "StringAssert"
            );
            if is_direct {
                let (literals, site_key) = literals_and_key(c, src, LITERALS);
                let subject_idents = c
                    .child_by_field_name("arguments")
                    .map(ident_count)
                    .unwrap_or(0);
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
            let Some(chain_root) = should_chain_root(c, src) else {
                return;
            };
            let (literals, site_key) = literals_and_key(c, src, LITERALS);
            let subject_idents = c
                .child_by_field_name("arguments")
                .map(ident_count)
                .unwrap_or(0)
                + ident_count(chain_root);
            assertions.push(AssertionSite {
                strength: strength_of(&callee),
                callee,
                site_key,
                literals,
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
using Xunit;
using FluentAssertions;

public class CalculatorTests
{
    [Fact]
    public void AddsTwoNumbers()
    {
        int result = Calculator.Add(1, 2);
        Assert.Equal(3, result);
        Assert.True(result > 0);
        result.Should().Be(3);
    }

    [Fact(Skip = "flaky on CI")]
    public void FlakyTest()
    {
        Assert.Equal(1, 1);
    }

    [Theory]
    [InlineData(1)]
    public void ParamTest(int x)
    {
        x.Should().BeGreaterThan(0);
    }

    public void Helper()
    {
        return;
    }
}
"#;

    #[test]
    fn finds_tests_assertions_and_skips() {
        let inv = extract(SRC, Language::CSharp);
        let names: Vec<&str> = inv.tests.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["AddsTwoNumbers", "FlakyTest", "ParamTest"]);

        let adds = &inv.tests[0];
        assert_eq!(adds.assertions.len(), 3);
        assert_eq!(adds.assertions[0].callee, "Equal");
        assert_eq!(adds.assertions[0].strength, Strength::Exact);
        assert_eq!(adds.assertions[0].literals, ["3"]);
        assert_eq!(adds.assertions[1].callee, "True");
        assert_eq!(adds.assertions[1].strength, Strength::Existence);
        assert_eq!(adds.assertions[2].callee, "Be");
        assert_eq!(adds.assertions[2].strength, Strength::Exact);
        assert_eq!(adds.assertions[2].literals, ["3"]);

        let flaky = &inv.tests[1];
        assert!(flaky.disabled);
        assert_eq!(flaky.skip_markers, 1);

        let param = &inv.tests[2];
        assert_eq!(param.assertions.len(), 1);
        assert_eq!(param.assertions[0].callee, "BeGreaterThan");
        assert_eq!(param.assertions[0].strength, Strength::Relational);
    }

    #[test]
    fn non_attributed_methods_are_not_tests() {
        let inv = extract(
            "public class X { public void Helper() { Assert.Equal(1, 1); } }",
            Language::CSharp,
        );
        assert!(inv.is_empty());
    }

    #[test]
    fn pure_literal_subject_is_a_tautology() {
        let inv = extract(
            r#"public class X {
                [Fact]
                public void T() {
                    Assert.Equal(1, 1);
                    Assert.Equal(x.Size(), 2);
                }
            }"#,
            Language::CSharp,
        );
        let t = &inv.tests[0];
        assert_eq!(t.assertions[0].subject_idents, 0);
        assert!(t.assertions[1].subject_idents > 0);
    }

    #[test]
    fn fluent_chain_only_counts_outermost_call() {
        let inv = extract(
            r#"public class X {
                [Fact]
                public void T() {
                    name.Should().StartWith("foo");
                }
            }"#,
            Language::CSharp,
        );
        let t = &inv.tests[0];
        assert_eq!(t.assertions.len(), 1);
        assert_eq!(t.assertions[0].callee, "StartWith");
        assert_eq!(t.assertions[0].strength, Strength::Relational);
        assert!(t.assertions[0].subject_idents > 0);
    }
}
