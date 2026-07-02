//! Tokenisation — port of `engine/argot/tokenize.py`.
//!
//! Parses source with tree-sitter and returns the flat list of leaf tokens
//! (nodes with no children and a non-empty byte span), in pre-order. Grammar
//! versions are pinned in `Cargo.toml` to match the Python engine so parse
//! trees — and therefore tokens — are identical.

use crate::dataset::{Language, Token};
use tree_sitter::{Node, Parser};

/// Map a file path to its language, mirroring Python's
/// `EXTENSION_TO_LANGUAGE` over `Path(path).suffix.lower()`.
pub fn language_for_path(path: &str) -> Option<Language> {
    match path_suffix_lower(path).as_str() {
        ".ts" | ".tsx" => Some(Language::Typescript),
        ".js" | ".jsx" => Some(Language::Javascript),
        ".py" => Some(Language::Python),
        ".cs" => Some(Language::Csharp),
        _ => None,
    }
}

fn basename(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

/// Reproduce `pathlib.PurePath.suffix` (lowercased): the substring from the
/// last dot in the basename, but only when that dot is neither the first nor
/// the last character. Returns `""` otherwise.
fn path_suffix_lower(path: &str) -> String {
    let name = basename(path);
    match name.rfind('.') {
        Some(i) if i > 0 && i < name.len() - 1 => name[i..].to_ascii_lowercase(),
        _ => String::new(),
    }
}

fn ts_language(lang: Language) -> tree_sitter::Language {
    match lang {
        Language::Typescript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Javascript => tree_sitter_javascript::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::Csharp => tree_sitter_c_sharp::LANGUAGE.into(),
    }
}

fn collect_tokens(node: Node, source: &[u8], out: &mut Vec<Token>) {
    if node.child_count() == 0 {
        let range = node.byte_range();
        if !range.is_empty() {
            let text = String::from_utf8_lossy(&source[range]).into_owned();
            out.push(Token {
                text,
                node_type: node.kind().to_string(),
                start_line: node.start_position().row,
                end_line: node.end_position().row,
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_tokens(child, source, out);
    }
}

/// Parse `source` and return a flat list of leaf tokens (`tokenize.py`).
pub fn tokenize(source: &[u8], lang: Language) -> Vec<Token> {
    let mut parser = Parser::new();
    parser
        .set_language(&ts_language(lang))
        .expect("tree-sitter grammar loads");
    let tree = parser.parse(source, None).expect("tree-sitter parse");
    let mut tokens = Vec::new();
    collect_tokens(tree.root_node(), source, &mut tokens);
    tokens
}

/// Tokenise the slice `source_lines[start_line..end_line]` (0-indexed, end
/// exclusive), joined with `\n`, then shift token line numbers to absolute.
///
/// Indices are clamped to `[0, len]` to mirror Python slice semantics (which
/// never panic on out-of-range bounds).
pub fn tokenize_lines(
    source_lines: &[&str],
    lang: Language,
    start_line: usize,
    end_line: usize,
) -> Vec<Token> {
    let len = source_lines.len();
    let start = start_line.min(len);
    let end = end_line.min(len).max(start);
    let slice = source_lines[start..end].join("\n");
    let mut raw = tokenize(slice.as_bytes(), lang);
    for t in &mut raw {
        t.start_line += start;
        t.end_line += start;
    }
    raw
}

#[cfg(test)]
mod tests {
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
        assert_eq!(language_for_path("a.rs"), None);
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
}
