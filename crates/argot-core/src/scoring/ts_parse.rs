//! Thread-local reused tree-sitter parsers for the scoring hot path.
//!
//! Creating a `Parser` and reloading the grammar on every call is pure
//! overhead — the Python engine keeps module-level parsers (`_PY_PARSER`,
//! `_TS_PARSER`) and reuses them. In scoring, `extract_imports`,
//! `extract_callees`, `prose_line_ranges`, and typicality each parse per hunk,
//! plus per corpus file at fit; reusing one parser per language per thread
//! removes thousands of parser allocations. Trees are owned, so reuse is safe.

use crate::scoring::adapters::Language;
use std::cell::RefCell;
use tree_sitter::{Parser, Tree};

fn new_parser(language: Language) -> Parser {
    let mut parser = Parser::new();
    let lang: tree_sitter::Language = match language {
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::Typescript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
    };
    parser
        .set_language(&lang)
        .expect("tree-sitter grammar loads");
    parser
}

thread_local! {
    static PY_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Python));
    static TS_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Typescript));
    static GO_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Go));
}

/// Parse `source` with a reused per-thread parser for `language`.
pub fn parse(source: &str, language: Language) -> Option<Tree> {
    match language {
        Language::Python => PY_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Typescript => TS_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Go => GO_PARSER.with(|p| p.borrow_mut().parse(source, None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_python_into_a_module_with_no_errors() {
        let tree = parse("def foo():\n    pass\n", Language::Python).expect("parse succeeds");
        assert_eq!(tree.root_node().kind(), "module");
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn parses_valid_typescript_into_a_program_with_no_errors() {
        let tree =
            parse("function foo(): void {}\n", Language::Typescript).expect("parse succeeds");
        assert_eq!(tree.root_node().kind(), "program");
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn parses_valid_go_into_a_source_file() {
        let tree = parse("package main\n\nfunc main() {}\n", Language::Go).expect("parse succeeds");
        assert_eq!(tree.root_node().kind(), "source_file");
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn empty_source_still_parses_to_an_empty_tree() {
        let tree = parse("", Language::Python).expect("empty input is a valid (empty) parse");
        assert_eq!(tree.root_node().kind(), "module");
        assert_eq!(tree.root_node().child_count(), 0);
    }

    #[test]
    fn syntactically_broken_source_still_returns_a_tree_flagged_with_an_error() {
        // tree-sitter is error-tolerant: parse() still returns Some(tree), but
        // the tree records that recovery was needed.
        let tree = parse("def foo(:\n", Language::Python).expect("parse never returns None");
        assert!(tree.root_node().has_error());
    }

    #[test]
    fn reusing_the_thread_local_parser_reflects_each_call_source_independently() {
        // The parser is reused across calls (thread_local); each call must
        // still produce a tree over exactly its own `source`, with no leakage
        // from the previous call on the same thread.
        let first = parse("def a():\n    pass\n", Language::Python).unwrap();
        let second = parse("def b():\n    return 1\n", Language::Python).unwrap();
        assert_eq!(first.root_node().byte_range(), 0..18);
        assert_eq!(second.root_node().byte_range(), 0..22);
    }

    #[test]
    fn python_and_typescript_parsers_are_independent() {
        // Interleave languages on the same thread and confirm neither
        // thread-local parser's state bleeds into the other's result.
        let py = parse("x = 1\n", Language::Python).unwrap();
        let ts = parse("let x: number = 1;\n", Language::Typescript).unwrap();
        assert_eq!(py.root_node().kind(), "module");
        assert_eq!(ts.root_node().kind(), "program");
        assert!(!py.root_node().has_error());
        assert!(!ts.root_node().has_error());
    }
}
