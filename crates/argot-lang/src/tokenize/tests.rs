use super::*;

#[test]
fn suffix_matches_pathlib() {
    assert_eq!(path_suffix_lower("util.ts"), ".ts");
    assert_eq!(path_suffix_lower("a/b/c.PY"), ".py");
    assert_eq!(path_suffix_lower("foo.tar.gz"), ".gz");
    assert_eq!(path_suffix_lower(".bashrc"), "");
    assert_eq!(path_suffix_lower("trailingdot."), "");
    assert_eq!(path_suffix_lower("noext"), "");
}

#[test]
fn language_routing() {
    assert_eq!(language_for_path("a.py"), Some(Language::Python));
    assert_eq!(language_for_path("a.tsx"), Some(Language::Typescript));
    assert_eq!(language_for_path("a.jsx"), Some(Language::Javascript));
    assert_eq!(language_for_path("a.go"), Some(Language::Go));
    assert_eq!(language_for_path("a.rs"), Some(Language::Rust));
    assert_eq!(language_for_path("a.c"), Some(Language::C));
    assert_eq!(language_for_path("a.h"), Some(Language::C));
    assert_eq!(language_for_path("a.java"), Some(Language::Java));
    assert_eq!(language_for_path("a.cs"), Some(Language::Csharp));
    assert_eq!(language_for_path("a.php"), Some(Language::Php));
    assert_eq!(language_for_path("a.cpp"), Some(Language::Cpp));
    assert_eq!(language_for_path("a.hpp"), Some(Language::Cpp));
    assert_eq!(language_for_path("a.rb"), Some(Language::Ruby));
    assert_eq!(language_for_path("a.unknown"), None);
}

#[test]
fn python_keyword_token_kinds_equal_text() {
    // For anonymous tokens like `if`/`)`, tree-sitter's node kind equals
    // the literal text — matching the Python golden output.
    let toks = tokenize(b"if x:\n    pass\n", Language::Python);
    let first = &toks[0];
    assert_eq!(first.text, "if");
    assert_eq!(first.node_type, "if");
    assert_eq!(first.start_line, 0);
}

#[test]
fn a_deeply_nested_tree_does_not_exhaust_the_stack() {
    // Tree depth is a property of the input. tree-sitter's own generated
    // `parser.c` — ~10 MB of nested initialisers, and a file argot's own
    // argot.toml excludes — overflowed the stack of a recursive walk and
    // aborted `argot extract` outright. 20 000 levels is far past what any
    // recursive version survives on a 2 MiB test-thread stack.
    let depth = 20_000;
    let mut src = String::from("int x = ");
    src.push_str(&"(".repeat(depth));
    src.push('1');
    src.push_str(&")".repeat(depth));
    src.push_str(";\n");

    let toks = tokenize(src.as_bytes(), Language::C);
    // `(` × depth, the literal, `)` × depth, plus `int x = ` and `;`.
    assert!(
        toks.len() > 2 * depth,
        "every leaf must be emitted: {} tokens for depth {depth}",
        toks.len()
    );
}

#[test]
fn tokens_come_out_in_document_order() {
    // The walk is iterative; document order is the property that could quietly
    // break if the child stack were pushed the wrong way round.
    let toks = tokenize(b"int a = 1; int b = 2;\n", Language::C);
    let text: Vec<&str> = toks.iter().map(|t| t.text.as_str()).collect();
    assert_eq!(
        text,
        vec!["int", "a", "=", "1", ";", "int", "b", "=", "2", ";"],
        "leaves must be emitted left to right"
    );
}
