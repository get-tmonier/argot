//! Tokenisation.
//!
//! Parses source with tree-sitter and returns the flat list of leaf tokens
//! (nodes with no children and a non-empty byte span), in pre-order. Grammar
//! versions are kept current in `Cargo.toml`; parse-tree — and therefore
//! token — stability is guarded by the golden suites, not by freezing versions.

use crate::dataset::{Language, Token};
use tree_sitter::{Node, Parser};

/// Map a file path to its language, keyed by `Path(path).suffix.lower()`.
pub fn language_for_path(path: &str) -> Option<Language> {
    match path_suffix_lower(path).as_str() {
        ".ts" | ".tsx" => Some(Language::Typescript),
        ".js" | ".jsx" => Some(Language::Javascript),
        ".py" => Some(Language::Python),
        ".go" => Some(Language::Go),
        ".rs" => Some(Language::Rust),
        ".c" | ".h" => Some(Language::C),
        ".java" => Some(Language::Java),
        ".cs" => Some(Language::Csharp),
        ".php" => Some(Language::Php),
        ".cpp" | ".cc" | ".hpp" | ".cxx" => Some(Language::Cpp),
        ".rb" => Some(Language::Ruby),
        ".pas" | ".pp" | ".dpr" | ".lpr" | ".inc" => Some(Language::Pascal),
        _ => None,
    }
}

/// [`language_for_path`], resolving the extensions the name alone cannot settle
/// (`.h` → C/C++, `.inc` → Pascal/C) against what the repository writes. Routes
/// through [`crate::ext::ext_to_lang_ctx`], the one table that decides this, so
/// the dataset labels match how calibrate and check route the same file.
pub fn language_for_path_ctx(path: &str, langs: crate::ext::RepoLangs) -> Option<Language> {
    match crate::ext::ext_to_lang_ctx(&crate::ext::extension(path), langs)? {
        "typescript" => Some(Language::Typescript),
        "javascript" => Some(Language::Javascript),
        "python" => Some(Language::Python),
        "go" => Some(Language::Go),
        "rust" => Some(Language::Rust),
        "c" => Some(Language::C),
        "java" => Some(Language::Java),
        "csharp" => Some(Language::Csharp),
        "php" => Some(Language::Php),
        "cpp" => Some(Language::Cpp),
        "ruby" => Some(Language::Ruby),
        "pascal" => Some(Language::Pascal),
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
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::C => tree_sitter_c::LANGUAGE.into(),
        Language::Java => tree_sitter_java::LANGUAGE.into(),
        Language::Csharp => tree_sitter_c_sharp::LANGUAGE.into(),
        Language::Php => tree_sitter_php::LANGUAGE_PHP.into(),
        Language::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        Language::Ruby => tree_sitter_ruby::LANGUAGE.into(),
        Language::Pascal => tree_sitter_pascal::LANGUAGE.into(),
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
mod tests;
