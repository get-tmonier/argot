//! Pascal test inventory: FPCUnit / DUnit `procedure Test*` methods and their
//! `Assert*` / `Check*` assertions, plus DUnitX `Assert.*` calls and FPCUnit
//! `Ignore(...)` skip markers.
//!
//! The dominant FreePascal/Lazarus (FPCUnit) and Delphi (DUnit) convention is a
//! published `procedure TestFoo;` on a `TTestCase` descendant, so a test case is
//! a procedure/function DEFINITION whose (possibly `TClass.`-qualified) name
//! starts with `Test` (case-insensitive). DUnitX tests that rely solely on a
//! `[Test]` attribute without a `Test` name prefix are not detected — a known,
//! ecosystem-honest gap outside the FPC/Lazarus target.

use super::{ident_count, literals_and_key, node_text, walk_named, words_of};
use super::{AssertionSite, Strength, TestCase, TestInventory};

const LITERALS: &[&str] = &[
    "literalNumber",
    "literalString",
    "literalChar",
    "kTrue",
    "kFalse",
    "kNil",
];

/// Dotted callee of an `exprCall`: `AssertEquals` (bare identifier) or
/// `Assert.AreEqual` (an `exprDot` chain).
fn call_callee(call: tree_sitter::Node, src: &str) -> Option<String> {
    let mut entity = call.child_by_field_name("entity")?;
    let mut parts: Vec<String> = Vec::new();
    while entity.kind() == "exprDot" {
        let rhs = entity.child_by_field_name("rhs")?;
        parts.insert(0, node_text(rhs, src).to_string());
        entity = entity.child_by_field_name("lhs")?;
    }
    if entity.kind() == "identifier" {
        parts.insert(0, node_text(entity, src).to_string());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("."))
    }
}

/// The method/procedure name a `defProc` binds — the `rhs` of a `TClass.Method`
/// `genericDot`, or the bare identifier.
fn defproc_name(defproc: tree_sitter::Node, src: &str) -> Option<String> {
    let header = defproc.child_by_field_name("header")?;
    let name = header.child_by_field_name("name")?;
    match name.kind() {
        "genericDot" => name
            .child_by_field_name("rhs")
            .map(|r| node_text(r, src).to_string()),
        _ => Some(node_text(name, src).to_string()),
    }
}

/// Recognise an assertion callee and classify how much it pins down. `None` for
/// non-assertion calls. Covers FPCUnit/DUnit `Assert*`/`Check*`/`Fail` and
/// DUnitX `Assert.AreEqual`/`Assert.IsTrue`/… (matched on the leading `Assert`
/// receiver or the method name).
fn assertion_strength(callee: &str) -> Option<Strength> {
    let leading = callee.split('.').next().unwrap_or(callee);
    let last = callee.rsplit('.').next().unwrap_or(callee);
    let l = last.to_ascii_lowercase();
    let recognized = l.starts_with("assert")
        || l.starts_with("check")
        || l == "fail"
        || leading.eq_ignore_ascii_case("assert");
    if !recognized {
        return None;
    }
    let strength = if l.contains("equal") || l.contains("same") {
        Strength::Exact
    } else if l.contains("greater")
        || l.contains("less")
        || l.contains("contains")
        || l.contains("match")
        || l.contains("raise")
    {
        Strength::Relational
    } else {
        Strength::Existence
    };
    Some(strength)
}

/// Scan a test body for `Ignore(...)` skip markers and assertion sites.
fn scan_body(
    n: tree_sitter::Node,
    src: &str,
    skip_markers: &mut usize,
    assertions: &mut Vec<AssertionSite>,
) {
    walk_named(n, &mut |c| {
        if c.kind() != "exprCall" {
            return;
        }
        let Some(callee) = call_callee(c, src) else {
            return;
        };
        let last = callee.rsplit('.').next().unwrap_or(&callee);
        if last.eq_ignore_ascii_case("ignore") {
            *skip_markers += 1;
            return;
        }
        let Some(strength) = assertion_strength(&callee) else {
            return;
        };
        let (literals, site_key) = literals_and_key(c, src, LITERALS);
        let subject_idents = c.child_by_field_name("args").map(ident_count).unwrap_or(0);
        assertions.push(AssertionSite {
            callee,
            site_key,
            literals,
            strength,
            line: c.start_position().row + 1,
            subject_idents,
        });
    });
}

pub(super) fn extract(root: tree_sitter::Node, src: &str) -> TestInventory {
    let mut inv = TestInventory::default();
    walk_named(root, &mut |n| {
        if n.kind() != "defProc" {
            return;
        }
        let Some(name) = defproc_name(n, src) else {
            return;
        };
        // FPCUnit/DUnit convention: a published `procedure Test*;` on a test
        // case. Case-insensitive (Pascal identifiers are case-insensitive).
        if !name.to_ascii_lowercase().starts_with("test") {
            return;
        }
        let mut skip_markers = 0usize;
        let mut assertions = Vec::new();
        if let Some(body) = n.child_by_field_name("body") {
            scan_body(body, src, &mut skip_markers, &mut assertions);
        }
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
unit MyTests;
interface
uses fpcunit, testregistry;
type
  TMathTests = class(TTestCase)
  published
    procedure TestAddition;
    procedure TestFlaky;
  end;
implementation

procedure TMathTests.TestAddition;
var
  R: Integer;
begin
  R := Add(1, 2);
  AssertEquals('sum', 3, R);
  AssertTrue(R > 0);
end;

procedure TMathTests.TestFlaky;
begin
  Ignore('flaky on CI');
  Assert.AreEqual(1, 1);
end;

initialization
  RegisterTest(TMathTests);
end.
"#;

    #[test]
    fn finds_fpcunit_test_procedures() {
        let inv = extract(SRC, Language::Pascal);
        let names: Vec<&str> = inv.tests.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["TestAddition", "TestFlaky"]);
    }

    #[test]
    fn assertions_carry_strength_and_literals() {
        let inv = extract(SRC, Language::Pascal);
        let add = &inv.tests[0];
        assert_eq!(add.assertions.len(), 2, "{:?}", add.assertions);
        assert_eq!(add.assertions[0].callee, "AssertEquals");
        assert_eq!(add.assertions[0].strength, Strength::Exact);
        assert!(add.assertions[0].literals.contains(&"3".to_string()));
        assert_eq!(add.assertions[1].callee, "AssertTrue");
        assert_eq!(add.assertions[1].strength, Strength::Existence);
    }

    #[test]
    fn ignore_is_a_skip_marker_and_dunitx_assert_is_detected() {
        let inv = extract(SRC, Language::Pascal);
        let flaky = &inv.tests[1];
        assert_eq!(flaky.skip_markers, 1);
        // DUnitX `Assert.AreEqual` recognised via the leading `Assert` receiver.
        assert_eq!(flaky.assertions.len(), 1);
        assert_eq!(flaky.assertions[0].callee, "Assert.AreEqual");
        assert_eq!(flaky.assertions[0].strength, Strength::Exact);
        // Pure-literal subject → tautology (no identifier operands).
        assert_eq!(flaky.assertions[0].subject_idents, 0);
    }
}
