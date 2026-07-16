//! Thread-local reused tree-sitter parsers for the scoring hot path.
//!
//! Creating a `Parser` and reloading the grammar on every call is pure
//! overhead — the Python engine keeps module-level parsers (`_PY_PARSER`,
//! `_TS_PARSER`) and reuses them. In scoring, `extract_imports`,
//! `extract_callees`, `prose_line_ranges`, and typicality each parse per hunk,
//! plus per corpus file at fit; reusing one parser per language per thread
//! removes thousands of parser allocations. Trees are owned, so reuse is safe.

use crate::adapters::Language;
use std::cell::RefCell;
use tree_sitter::{Parser, Tree};

/// The tree-sitter grammar for a scoring language — the one grammar table
/// every parse in the workspace routes through (scorers, adapters, and the
/// scripted rules' `ts_query` host call).
pub fn ts_language(language: Language) -> tree_sitter::Language {
    match language {
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::Typescript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Javascript => tree_sitter_javascript::LANGUAGE.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::C => tree_sitter_c::LANGUAGE.into(),
        Language::Java => tree_sitter_java::LANGUAGE.into(),
        Language::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
        Language::Php => tree_sitter_php::LANGUAGE_PHP.into(),
        Language::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        Language::Ruby => tree_sitter_ruby::LANGUAGE.into(),
    }
}

fn new_parser(language: Language) -> Parser {
    let mut parser = Parser::new();
    let lang: tree_sitter::Language = ts_language(language);
    parser
        .set_language(&lang)
        .expect("tree-sitter grammar loads");
    parser
}

thread_local! {
    static PY_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Python));
    static TS_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Typescript));
    static JS_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Javascript));
    static GO_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Go));
    static RUST_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Rust));
    static C_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::C));
    static JAVA_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Java));
    static CS_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::CSharp));
    static PHP_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Php));
    static CPP_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Cpp));
    static RB_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Ruby));
}

/// Parse `source` with a reused per-thread parser for `language`.
pub fn parse(source: &str, language: Language) -> Option<Tree> {
    match language {
        Language::Python => PY_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Typescript => TS_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Javascript => JS_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Go => GO_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Rust => RUST_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::C => C_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Java => JAVA_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::CSharp => CS_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Php => PHP_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Cpp => CPP_PARSER.with(|p| p.borrow_mut().parse(source, None)),
        Language::Ruby => RB_PARSER.with(|p| p.borrow_mut().parse(source, None)),
    }
}

/// The direct children of `node`, in left-to-right tree order.
///
/// tree-sitter's index accessor `Node::child(i)` takes a `u32` and costs
/// log(i) per call; a cursor walk is the idiomatic linear traversal and keeps
/// callers free of `usize`→`u32` index casts. Reverse the result for the
/// stack-based pre-order DFS the scorers use (push children reversed so they
/// pop left-to-right).
pub fn child_nodes<'t>(node: tree_sitter::Node<'t>) -> Vec<tree_sitter::Node<'t>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).collect()
}

/// The named direct children of `node`, in tree order (skips anonymous tokens).
pub fn named_child_nodes<'t>(node: tree_sitter::Node<'t>) -> Vec<tree_sitter::Node<'t>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

#[cfg(test)]
mod tests;
