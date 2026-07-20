//! Per-version test inventories — the AST facts the test-integrity detector
//! diffs (`--features integrity`).
//!
//! For one file version, [`extract`] returns every test case the language's
//! ecosystem conventions define (pytest/unittest functions, `it/test` blocks,
//! `#[test]` fns, `TestXxx(t *testing.T)`, `@Test` methods, xUnit attributes,
//! specs, PHPUnit methods, gtest/catch2 macros …), and for each test its
//! **assertion sites**, **skip/disable markers**, and **expected literals**.
//! The detector ([`super::integrity`]) diffs the old and new inventories of a
//! changed file into gaming events; nothing here looks at the diff itself.
//!
//! Everything in this module is a **language-ecosystem fact** — the same tier
//! as the tree-sitter grammars. Repo-specific knowledge (learned test paths,
//! per-event gates) lives in the fit artifact, never here.

use std::collections::HashSet;

use argot_lang::adapters::Language;
use argot_lang::ts_parse;

mod c;
mod cpp;
mod csharp;
mod go;
mod java;
mod pascal;
mod php;
mod python;
mod ruby;
mod rust;
mod typescript;

/// How much an assertion pins down: an exact expectation beats a relational
/// or containment check, which beats mere existence/truthiness. A same-site
/// downgrade across a diff is a *weakening* event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Strength {
    /// Not-null / truthy / no-throw / bare fail-if-reached.
    Existence = 0,
    /// Relational or containment (`>=`, contains, matches).
    Relational = 1,
    /// Exact expectation (equality against a specific value).
    Exact = 2,
}

/// One assertion call/statement inside a test.
#[derive(Debug, Clone)]
pub struct AssertionSite {
    /// The assertion's callee/matcher name (`assertEqual`, `toBe`,
    /// `assert_eq`, `require.NoError`, …) — evidence vocabulary.
    pub callee: String,
    /// The assertion text with literal spans blanked to `▯` and whitespace
    /// collapsed — the site's identity for retarget detection (same site,
    /// different literal) and for changeset-wide moved-assertion accounting.
    pub site_key: String,
    /// The literal argument texts, in order — the expected values.
    pub literals: Vec<String>,
    pub strength: Strength,
    /// 1-indexed line of the assertion in its file version.
    pub line: usize,
    /// Identifier leaves in the assertion's subject (operands/arguments, not
    /// the callee). 0 == a pure-literal subject == a tautology that can never
    /// fail (`assert 1 == 1`, `expect(true).toBe(true)`).
    pub subject_idents: usize,
}

/// One test case (function / block) in a file version.
#[derive(Debug, Clone)]
pub struct TestCase {
    /// The test's name (function name, or the spec title for block-style
    /// frameworks).
    pub name: String,
    /// 1-indexed line the test starts on.
    pub line: usize,
    /// The test is written in a disabled form (`xit`, `it.skip`,
    /// gtest `DISABLED_` prefix, `#[ignore]`, …).
    pub disabled: bool,
    /// Count of skip/xfail/ignore markers attached to or inside the test.
    pub skip_markers: usize,
    pub assertions: Vec<AssertionSite>,
    /// Word set of the body — rename/move detection across the changeset.
    pub body_words: HashSet<String>,
    /// Body height in lines — gutting detection.
    pub body_lines: usize,
}

/// Every test fact extracted from one file version.
#[derive(Debug, Clone, Default)]
pub struct TestInventory {
    pub tests: Vec<TestCase>,
}

impl TestInventory {
    pub fn is_empty(&self) -> bool {
        self.tests.is_empty()
    }
}

/// Extract the test inventory of one file version. Unparseable or
/// test-free sources yield an empty inventory (the detector's graceful
/// no-op) — never an error.
pub fn extract(source: &str, language: Language) -> TestInventory {
    let Some(tree) = ts_parse::parse(source, language) else {
        return TestInventory::default();
    };
    let root = tree.root_node();
    match language {
        Language::Python => python::extract(root, source),
        Language::Typescript | Language::Javascript => typescript::extract(root, source),
        Language::Rust => rust::extract(root, source),
        Language::Go => go::extract(root, source),
        Language::Java => java::extract(root, source),
        Language::CSharp => csharp::extract(root, source),
        Language::Ruby => ruby::extract(root, source),
        Language::Php => php::extract(root, source),
        Language::C => c::extract(root, source),
        Language::Cpp => cpp::extract(root, source),
        Language::Pascal => pascal::extract(root, source),
    }
}

/// Is this path a test file by the language ecosystem's naming conventions?
/// (Content is the other half: a file whose inventory has tests is a test
/// carrier regardless of path — Rust unit tests live inside prod files.)
pub fn is_test_path(path: &str, language: Language) -> bool {
    let lower = path.to_lowercase();
    let base = lower.rsplit('/').next().unwrap_or(&lower);
    let in_test_dir = lower.split('/').any(|c| {
        c == "test"
            || c == "tests"
            || c == "testing"
            || c == "__tests__"
            || c == "spec"
            || c == "specs"
    });
    match language {
        Language::Python => {
            in_test_dir
                || base.starts_with("test_")
                || base.ends_with("_test.py")
                || base == "conftest.py"
        }
        Language::Typescript | Language::Javascript => {
            in_test_dir || base.contains(".test.") || base.contains(".spec.")
        }
        Language::Go => base.ends_with("_test.go"),
        // Rust integration tests live under tests/; unit tests are in-file
        // (content-detected).
        Language::Rust => in_test_dir,
        Language::Java => {
            lower.contains("/src/test/")
                || base.ends_with("test.java")
                || base.ends_with("tests.java")
                || base.starts_with("test") && base.ends_with(".java") && lower.contains("/test/")
        }
        Language::CSharp => {
            in_test_dir
                || base.ends_with("test.cs")
                || base.ends_with("tests.cs")
                || lower.contains(".tests/")
                || lower.contains(".test/")
        }
        Language::Ruby => {
            base.ends_with("_spec.rb")
                || base.ends_with("_test.rb")
                || base.starts_with("test_") && base.ends_with(".rb")
                || in_test_dir
        }
        Language::Php => in_test_dir || base.ends_with("test.php"),
        Language::C | Language::Cpp => {
            in_test_dir
                || base.contains("_test.")
                || base.contains("test_")
                || base.contains("_unittest.")
        }
        // FPCUnit/DUnit units are conventionally `*Tests.pas` / `test*.pas`, or
        // live under a tests/ tree; content detection covers the rest.
        Language::Pascal => {
            in_test_dir
                || base.starts_with("test")
                || base.ends_with("test.pas")
                || base.ends_with("tests.pas")
                || base.ends_with("test.pp")
                || base.ends_with("tests.pp")
                || base.ends_with("test.dpr")
                || base.ends_with("tests.dpr")
        }
    }
}

/// Language by file extension — the integrity tier's own mapping (`.h` maps
/// to C; header-only C++ tests are rare and degrade gracefully to empty
/// inventories).
pub fn language_for_path(path: &str) -> Option<Language> {
    let ext = std::path::Path::new(path).extension()?.to_str()?;
    match ext {
        "py" => Some(Language::Python),
        "ts" | "tsx" | "mts" | "cts" => Some(Language::Typescript),
        "js" | "jsx" | "mjs" | "cjs" => Some(Language::Javascript),
        "rs" => Some(Language::Rust),
        "go" => Some(Language::Go),
        "java" => Some(Language::Java),
        "cs" => Some(Language::CSharp),
        "rb" => Some(Language::Ruby),
        "php" => Some(Language::Php),
        "c" | "h" => Some(Language::C),
        "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => Some(Language::Cpp),
        "pas" | "pp" | "dpr" | "lpr" | "inc" => Some(Language::Pascal),
        _ => None,
    }
}

/// The symbols a file version DEFINES (functions, methods, types) — the
/// integrity detector's "does the production code this test exercised still
/// exist?" check. Generic over the def-node kinds of every supported grammar;
/// misses degrade conservatively (fewer defs → deletion looks legitimate).
pub fn defined_symbols(source: &str, language: Language) -> HashSet<String> {
    let mut defs = HashSet::new();
    let Some(tree) = ts_parse::parse(source, language) else {
        return defs;
    };
    walk_named(tree.root_node(), &mut |n| {
        // TS/JS `const foo = () => {}` / `const foo = function () {}` — the
        // dominant modern export form. The callable's name is the declarator
        // binding, not a `name` field on the (anonymous) function node, so the
        // node-kind arm below never sees it. Without this, a co-located test
        // exercising `foo` looks like it covers nothing and its deletion reads
        // as legitimate (B4: test-deleted blind to arrow/const exports).
        if n.kind() == "variable_declarator" {
            let is_callable = n
                .child_by_field_name("value")
                .is_some_and(|v| matches!(v.kind(), "arrow_function" | "function_expression"));
            if is_callable {
                if let Some(name) = n.child_by_field_name("name") {
                    if name.kind().ends_with("identifier") {
                        let t = node_text(name, source);
                        if t.len() > 2 {
                            defs.insert(t.to_string());
                        }
                    }
                }
            }
            return;
        }
        // Pascal: a `defProc` (implementation body) binds its name on the
        // header's `name` — a `genericDot` (`TClass.Method`, the method is its
        // `rhs`) or a bare identifier; `declType`/`declProc` expose a direct
        // `name`. None match the generic node-kind arm below, so without this a
        // test's deleted subject never looks "still defined" and `test-deleted`
        // can't fire for Pascal.
        if matches!(n.kind(), "defProc" | "declProc" | "declType") {
            let name_node = if n.kind() == "defProc" {
                n.child_by_field_name("header")
                    .and_then(|h| h.child_by_field_name("name"))
            } else {
                n.child_by_field_name("name")
            };
            if let Some(name) = name_node {
                let ident = if name.kind() == "genericDot" {
                    name.child_by_field_name("rhs")
                } else {
                    Some(name)
                };
                if let Some(id) = ident {
                    let t = node_text(id, source);
                    if t.len() > 2 {
                        defs.insert(t.to_string());
                    }
                }
            }
            return;
        }
        if !matches!(
            n.kind(),
            "function_definition"
                | "function_declaration"
                | "function_item"
                | "method_declaration"
                | "method_definition"
                | "class_definition"
                | "class_declaration"
                | "struct_item"
                | "enum_item"
                | "trait_item"
                | "type_spec"
                | "interface_declaration"
                | "method"
                | "singleton_method"
                | "class"
                | "module"
                | "function_expression"
        ) {
            return;
        }
        // Named defs expose a `name` field; C/C++ function defs descend
        // through the declarator chain instead.
        if let Some(name) = n.child_by_field_name("name") {
            if name.kind().ends_with("identifier")
                || name.kind() == "constant"
                || name.kind() == "name"
            {
                let t = node_text(name, source);
                if t.len() > 2 {
                    defs.insert(t.to_string());
                }
            }
            return;
        }
        let mut d = n.child_by_field_name("declarator");
        while let Some(cur) = d {
            if cur.kind().ends_with("identifier") {
                let t = node_text(cur, source);
                if t.len() > 2 {
                    defs.insert(t.to_string());
                }
                break;
            }
            d = cur.child_by_field_name("declarator");
        }
    });
    defs
}

/// Can this assertion callee even EXPRESS a tautology? Only core-vocabulary
/// assertions whose argument semantics are known (the argument IS the
/// compared value / condition). A custom `assert*`-prefixed helper taking a
/// literal argument usually embeds its subject internally
/// (`assertRunFAIL("cmd")`) — flagging it as can-never-fail would be wrong.
pub fn tautology_capable(callee: &str) -> bool {
    // Strip generic arguments (`Equal<uint>` → `Equal`).
    let callee = callee.split('<').next().unwrap_or(callee).trim_end();
    const CORE: &[&str] = &[
        // language-level asserts
        "assert",
        "assert_eq",
        "assert_ne",
        "debug_assert",
        "debug_assert_eq",
        // xUnit-family comparisons/conditions
        "assertEqual",
        "assertNotEqual",
        "assertTrue",
        "assertFalse",
        "assertIs",
        "assertIsNot",
        "assertEquals",
        "assertNotEquals",
        "assertSame",
        "assertNotSame",
        "assertArrayEquals",
        "assertNull",
        "assertNotNull",
        // jest/vitest/chai/node
        "toBe",
        "toEqual",
        "toStrictEqual",
        "toBeTruthy",
        "toBeFalsy",
        "strictEqual",
        "deepEqual",
        "deepStrictEqual",
        "equal",
        "equals",
        "ok",
        "isTrue",
        "isFalse",
        // rspec/minitest
        "eq",
        "eql",
        "be",
        "be_truthy",
        "be_falsey",
        "assert_equal",
        "refute_equal",
        "must_equal",
        "wont_equal",
        // xunit/nunit/mstest/fluent
        "Equal",
        "NotEqual",
        "True",
        "False",
        "Same",
        "AreEqual",
        "AreSame",
        "IsTrue",
        "IsFalse",
        "Be",
        "BeTrue",
        "BeFalse",
        // php
        "assertSame",
        "assertCount",
        // go testify
        "assert.Equal",
        "require.Equal",
        "assert.True",
        "require.True",
        "assert.False",
        "require.False",
        "assert.Exactly",
        "require.Exactly",
        // FPCUnit / DUnit / DUnitX (Pascal, PascalCase — the CORE list above is
        // case-sensitive and its `assertEqual`/`assertTrue` entries never match
        // Pascal's `AssertEquals`/`AssertTrue`, so a Pascal `AssertEquals(1, 1)`
        // could not be flagged a tautology without these).
        "AssertEquals",
        "AssertTrue",
        "AssertFalse",
        "AssertSame",
        "AssertNull",
        "AssertNotNull",
        "CheckEquals",
        "CheckTrue",
        "CheckFalse",
        "CheckSame",
    ];
    CORE.contains(&callee)
        || callee.starts_with("EXPECT_")
        || callee.starts_with("ASSERT_")
        || callee == "REQUIRE"
        || callee == "CHECK"
        || callee.starts_with("TEST_ASSERT")
        || callee.contains("_eq")
        || callee.contains("_equal")
}

// ---------------------------------------------------------------- shared AST helpers

pub(crate) fn node_text<'a>(n: tree_sitter::Node, src: &'a str) -> &'a str {
    src.get(n.byte_range()).unwrap_or("")
}

/// Depth-first pre-order walk over named nodes.
pub(crate) fn walk_named<'t>(
    root: tree_sitter::Node<'t>,
    f: &mut impl FnMut(tree_sitter::Node<'t>),
) {
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        f(n);
        for c in argot_lang::ts_parse::named_child_nodes(n).into_iter().rev() {
            stack.push(c);
        }
    }
}

/// Identifier-ish word set of a text span (≥ 3 chars) — rename/move detection.
pub(crate) fn words_of(text: &str) -> HashSet<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| w.len() > 2)
        .map(|w| w.to_string())
        .collect()
}

/// Identifier leaves under a node — `0` under an assertion's subject means the
/// subject is pure literals (a tautology). Literals contribute no identifier
/// leaves by construction; interpolations inside template/f-strings do (an
/// interpolated expectation is not a tautology).
pub(crate) fn ident_count(n: tree_sitter::Node) -> usize {
    let mut count = 0usize;
    walk_named(n, &mut |c| {
        if c.kind().ends_with("identifier")
            || c.kind() == "self"
            || c.kind() == "variable_name"
            || c.kind() == "constant"
            || c.kind() == "name"
        {
            count += 1;
        }
    });
    count
}

/// Collect the literal descendants of an assertion node (outermost only) and
/// build its site key: the node text with each literal span blanked to `▯`
/// and whitespace collapsed.
pub(crate) fn literals_and_key(
    n: tree_sitter::Node,
    src: &str,
    literal_kinds: &[&str],
) -> (Vec<String>, String) {
    let mut lits: Vec<(usize, usize, String)> = Vec::new();
    walk_named(n, &mut |c| {
        if literal_kinds.contains(&c.kind())
            && !lits
                .iter()
                .any(|(s, e, _)| c.start_byte() >= *s && c.end_byte() <= *e)
        {
            lits.push((c.start_byte(), c.end_byte(), node_text(c, src).to_string()));
        }
    });
    lits.sort();
    let base = n.start_byte();
    let text = node_text(n, src);
    let mut key = String::new();
    let mut cursor = 0usize;
    for (s, e, _) in &lits {
        let (rs, re) = (s - base, e - base);
        if rs >= cursor && re <= text.len() {
            key.push_str(&text[cursor..rs]);
            key.push('▯');
            cursor = re;
        }
    }
    key.push_str(&text[cursor..]);
    let key: String = key.split_whitespace().collect::<Vec<_>>().join(" ");
    (lits.into_iter().map(|(_, _, t)| t).collect(), key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_unparseable_sources_yield_empty_inventories() {
        assert!(extract("", Language::Python).is_empty());
        assert!(extract("not really code {{{", Language::Go).is_empty());
    }

    #[test]
    fn defined_symbols_captures_ts_arrow_and_const_exports() {
        // The modern TS export forms bind the name on the declarator, not on the
        // anonymous function node (B4 — test-deleted was blind to these).
        let src = "export const getPath = (url: string) => url.slice(1);\n\
                   export const build = function () { return 1; };\n\
                   export function legacy() { return 2; }\n";
        let defs = defined_symbols(src, Language::Typescript);
        assert!(defs.contains("getPath"), "{defs:?}");
        assert!(defs.contains("build"), "{defs:?}");
        assert!(defs.contains("legacy"), "{defs:?}");
    }

    #[test]
    fn defined_symbols_and_tautology_cover_pascal() {
        // Pascal `defProc` binds its name on the header (a `TClass.Method`
        // genericDot or a bare identifier); `declType` names the class. Both must
        // land in `defined_symbols` so a deleted test's subject reads as "still
        // defined" and `test-deleted` can fire.
        let src = "unit U;\ninterface\ntype\n  TWidget = class(TObject)\n  end;\n\
                   implementation\n\
                   function TWidget.Area: Integer;\nbegin\n  Result := 1;\nend;\n\
                   procedure Helper;\nbegin\nend;\nend.\n";
        let defs = defined_symbols(src, Language::Pascal);
        assert!(
            defs.contains("Area"),
            "method via genericDot header: {defs:?}"
        );
        assert!(defs.contains("Helper"), "bare proc: {defs:?}");
        assert!(defs.contains("TWidget"), "type: {defs:?}");
        // PascalCase FPCUnit asserts must be tautology-capable (the CORE list is
        // case-sensitive; its camelCase entries never match Pascal).
        assert!(tautology_capable("AssertEquals"));
        assert!(tautology_capable("AssertTrue"));
        assert!(tautology_capable("CheckEquals"));
    }

    #[test]
    fn test_path_conventions_cover_the_ecosystems() {
        assert!(is_test_path("tests/test_main.py", Language::Python));
        assert!(is_test_path("pkg/conninfo.test.ts", Language::Typescript));
        assert!(is_test_path("pkg/api_test.go", Language::Go));
        assert!(is_test_path("tests/cli_tests.rs", Language::Rust));
        assert!(is_test_path("src/test/java/FooTest.java", Language::Java));
        assert!(is_test_path("Foo.Tests/BarTests.cs", Language::CSharp));
        assert!(is_test_path("spec/models/user_spec.rb", Language::Ruby));
        assert!(is_test_path("tests/Unit/UserTest.php", Language::Php));
        assert!(is_test_path("test/util_unittest.cc", Language::Cpp));
        assert!(!is_test_path("src/lib.rs", Language::Rust));
        assert!(!is_test_path("fastapi/routing.py", Language::Python));
        assert!(!is_test_path("src/index.ts", Language::Typescript));
    }

    #[test]
    fn words_of_keeps_identifiers_only() {
        let w = words_of("assert foo_bar == 12 && baz(qux)");
        assert!(w.contains("foo_bar"));
        assert!(w.contains("baz"));
        assert!(w.contains("qux"));
        assert!(!w.contains("12"));
    }
}
