//! Thread-local reused tree-sitter parsers for the scoring hot path.
//!
//! Creating a `Parser` and reloading the grammar on every call is pure
//! overhead — a reused parser per language avoids re-creating the grammar on
//! every call. In scoring, `extract_imports`,
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
        Language::Pascal => tree_sitter_pascal::LANGUAGE.into(),
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
    static PAS_PARSER: RefCell<Parser> = RefCell::new(new_parser(Language::Pascal));
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
        Language::Pascal => PAS_PARSER.with(|p| p.borrow_mut().parse(source, None)),
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

/// Whether `node` has an ancestor of one of `kinds` — a function declared
/// inside another function, whatever the language calls the node.
///
/// Sibling-span containment cannot answer this. Object Pascal's
/// `dbtrystringtoguid` declares two local procedures and the grammar does not
/// parse it at all — the whole rest of the unit becomes one `ERROR` node — so
/// the enclosing definition is never extracted and its children look top-level.
/// Passing `ERROR` among the kinds therefore also covers the honest case: a
/// callable recovered from inside a parse error has no known parent, and
/// nothing that depends on where it sits should claim to know.
pub fn has_ancestor_of_kind(node: tree_sitter::Node<'_>, kinds: &[&str]) -> bool {
    let mut cur = node.parent();
    while let Some(n) = cur {
        if kinds.contains(&n.kind()) {
            return true;
        }
        cur = n.parent();
    }
    false
}

/// What blanking a prose node costs: whole rows, or just the node's own span.
enum Prose {
    Rows(Vec<usize>),
    Span(usize, usize, usize),
}

/// Where a source's prose sits: rows to blank whole, plus `(row, col_start,
/// col_end)` spans to blank in place on rows that also carry code.
///
/// Masking by the *line* alone deletes the code beside a comment. That is
/// not hypothetical: `uses msedynload{,mseguiintf};` (Object Pascal, a unit
/// commented out in place) lost its whole `uses` clause, after which the
/// parser recovered the next type name as a module and the import stage
/// scored a dependency that does not exist — and the same shape under-reads
/// every language, a trailing `// note` after an import taking the import
/// with it. Masking by the *span* alone is no better: a multi-row node's
/// opening row would keep its delimiter after the rest is gone. So single-row
/// prose sharing its line with code is blanked in place, everything else by
/// the line.
#[derive(Debug, Default, Clone)]
pub struct ProseMask {
    pub rows: std::collections::HashSet<usize>,
    pub spans: Vec<(usize, usize, usize)>,
}

impl ProseMask {
    fn classify(source: &str, node: tree_sitter::Node<'_>) -> Prose {
        let (start, end) = (node.start_position(), node.end_position());
        if start.row == end.row {
            let line = source.split('\n').nth(start.row).unwrap_or("");
            let blank = |s: Option<&str>| s.is_none_or(|p| p.trim().is_empty());
            if !(blank(line.get(..start.column)) && blank(line.get(end.column..))) {
                return Prose::Span(start.row + 1, start.column, end.column);
            }
        }
        Prose::Rows((start.row + 1..=end.row + 1).collect())
    }

    /// Record one prose node.
    pub fn add(&mut self, source: &str, node: tree_sitter::Node<'_>) {
        match Self::classify(source, node) {
            Prose::Rows(rows) => self.rows.extend(rows),
            Prose::Span(r, a, b) => self.spans.push((r, a, b)),
        }
    }
}

#[cfg(test)]
mod tests;
