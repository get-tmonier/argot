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
