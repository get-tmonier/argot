//! Rust language adapter — the third language port beyond Python / TypeScript,
//! following the Go port as its structural reference.
//!
//! Structurally a sibling of `go.rs` / `python.rs`: manual `TreeCursor`
//! traversal over the pinned tree-sitter-rust grammar, adapted to Rust's node
//! kinds. Notes worth keeping in mind:
//! - Imports are `use` declarations; the module string is the crate root of the
//!   path (leftmost segment — `serde`, `tokio`, `std`), the analog of Python's
//!   top-level dotted-module pick. A `use foo::{a, b}` shares one crate root.
//! - `use crate::…` / `use self::…` / `use super::…` are repo-relative — they
//!   feed `internal_import_bindings`, not `extract_imports`, mirroring Python's
//!   relative-import handling.
//! - Data-dominance is computed against the Rust grammar directly (the shared
//!   `filters::data_dominant` is Python-grammar-bound), mirroring how the Go and
//!   TypeScript adapters roll their own.

use std::collections::HashSet;

use tree_sitter::{Node, Tree};

use super::Language;

use super::HEADER_LINE_LIMIT;

/// Ratio of data-literal rows above which a file is data-dominant.
/// Fraction of an array literal's elements that must be value literals for it to
/// count as static data.
const VALUE_DOMINANT_THRESHOLD: f64 = 0.8;

/// Node kinds that count as static value literals inside an array literal.
const RUST_VALUE_LITERAL_TYPES: &[&str] = &[
    "integer_literal",
    "float_literal",
    "string_literal",
    "raw_string_literal",
    "char_literal",
    "boolean_literal",
    "negative_literal",
    "array_expression",
    "tuple_expression",
    "unit_expression",
    "struct_expression",
];

/// Parse Rust `source` into a tree-sitter `Tree` (reused per-thread parser).
pub(crate) fn parse(source: &str) -> Tree {
    crate::scoring::ts_parse::parse(source, Language::Rust)
        .expect("tree-sitter parse never returns None without a timeout")
}

/// Collect every descendant of `node` (pre-order, excluding `node` itself).
fn descendants<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    fn walk<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            out.push(child);
            walk(child, out);
        }
    }
    for child in node.children(&mut cursor) {
        out.push(child);
        walk(child, &mut out);
    }
    out
}

/// Direct children of `node` as a `Vec`.
fn children<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).collect()
}

/// Direct named children of `node`.
fn named_children<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

/// Path-segment node kinds that can head a `use` path.
fn is_path_kind(kind: &str) -> bool {
    matches!(
        kind,
        "identifier" | "scoped_identifier" | "crate" | "self" | "super" | "metavariable"
    )
}

/// The crate-root segment node(s) of a `use`-clause, each paired with whether
/// the root is repo-relative (`crate` / `self` / `super`). A single grouped
/// `use foo::{a, b}` resolves to one root (`foo`); a prefix-less `use {a, b}`
/// yields one root per element.
fn use_crate_root_nodes<'a>(node: Node<'a>) -> Vec<(Node<'a>, bool)> {
    match node.kind() {
        "identifier" => vec![(node, false)],
        "crate" | "self" | "super" => vec![(node, true)],
        "scoped_identifier" => match node.child_by_field_name("path") {
            Some(path) => use_crate_root_nodes(path),
            None => node
                .child_by_field_name("name")
                .map(use_crate_root_nodes)
                .unwrap_or_default(),
        },
        "scoped_use_list" => match node.child_by_field_name("path") {
            Some(path) => use_crate_root_nodes(path),
            None => node
                .child_by_field_name("list")
                .map(use_crate_root_nodes)
                .unwrap_or_default(),
        },
        "use_as_clause" => node
            .child_by_field_name("path")
            .map(use_crate_root_nodes)
            .unwrap_or_default(),
        "use_wildcard" => named_children(node)
            .into_iter()
            .find(|c| is_path_kind(c.kind()))
            .map(use_crate_root_nodes)
            .unwrap_or_default(),
        "use_list" => named_children(node)
            .into_iter()
            .flat_map(use_crate_root_nodes)
            .collect(),
        _ => Vec::new(),
    }
}

/// The names a `use`-clause binds into scope (leaf segments / aliases). Used for
/// repo-relative imports, whose bound names are neighbourhood, not foreign.
fn use_bound_names(node: Node, source: &str) -> Vec<String> {
    match node.kind() {
        "identifier" => vec![node_text(node, source).to_string()],
        "scoped_identifier" => node
            .child_by_field_name("name")
            .map(|n| use_bound_names(n, source))
            .unwrap_or_default(),
        "use_as_clause" => node
            .child_by_field_name("alias")
            .map(|a| vec![node_text(a, source).to_string()])
            .unwrap_or_default(),
        "scoped_use_list" => node
            .child_by_field_name("list")
            .map(|l| use_bound_names(l, source))
            .unwrap_or_default(),
        "use_list" => named_children(node)
            .into_iter()
            .flat_map(|c| use_bound_names(c, source))
            .collect(),
        _ => Vec::new(),
    }
}

/// Insert every `identifier` node under `pattern` into `out`.
fn collect_pattern_ids(pattern: Node, source: &str, out: &mut HashSet<String>) {
    if pattern.kind() == "identifier" {
        out.insert(node_text(pattern, source).to_string());
    }
    for d in descendants(pattern) {
        if d.kind() == "identifier" {
            out.insert(node_text(d, source).to_string());
        }
    }
}

/// True if ≥80% of an array literal's elements are value literals. Empty
/// literals return true (conservative — allow).
fn is_rust_value_literal_dominant(array: Node) -> bool {
    let values: Vec<Node> = named_children(array)
        .into_iter()
        .filter(|c| c.kind() != "attribute_item")
        .collect();
    if values.is_empty() {
        return true;
    }
    let literal_count = values
        .iter()
        .filter(|v| RUST_VALUE_LITERAL_TYPES.contains(&v.kind()))
        .count();
    (literal_count as f64) / (values.len() as f64) >= VALUE_DOMINANT_THRESHOLD
}

/// The value expression of a `const`/`static` item, unwrapping a leading `&`
/// (`static T: &[…] = &[…]`).
fn item_data_array(item: Node) -> Option<Node> {
    let mut value = item.child_by_field_name("value")?;
    if value.kind() == "reference_expression" {
        value = value.child_by_field_name("value")?;
    }
    (value.kind() == "array_expression").then_some(value)
}

/// Add 0-indexed row spans of top-level data-literal `const`/`static` items to
/// `rows`.
fn collect_rust_data_rows(root: Node, rows: &mut HashSet<usize>) {
    for child in children(root) {
        if !matches!(child.kind(), "const_item" | "static_item") {
            continue;
        }
        if let Some(array) = item_data_array(child) {
            if is_rust_value_literal_dominant(array) {
                for r in child.start_position().row..=child.end_position().row {
                    rows.insert(r);
                }
            }
        }
    }
}

/// `RustAdapter` — the language adapter for `.rs` sources.
#[derive(Debug, Default)]
pub struct RustAdapter {
    noise: HashSet<String>,
}

impl RustAdapter {
    pub fn new() -> Self {
        Self {
            noise: NOISE.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Crate roots imported in `source` — the leftmost `use`-path segment
    /// (`serde`, `tokio`, `std`), never a repo-relative `crate`/`self`/`super`
    /// path.
    pub fn extract_imports(&self, source: &str) -> HashSet<String> {
        let tree = parse(source);
        let mut out = HashSet::new();
        for node in descendants(tree.root_node()) {
            if node.kind() != "use_declaration" {
                continue;
            }
            let Some(arg) = node.child_by_field_name("argument") else {
                continue;
            };
            for (root, relative) in use_crate_root_nodes(arg) {
                if relative {
                    continue;
                }
                let text = node_text(root, source);
                if !text.is_empty() {
                    out.insert(text.to_string());
                }
            }
        }
        out
    }

    /// Like `extract_imports` but each crate root carries its `(spec, line,
    /// col_start, col_end)` span. Line 1-indexed; columns 0-indexed byte
    /// offsets covering the root segment, end exclusive. Sorted by
    /// `(line, col_start, spec)`.
    pub fn extract_imports_with_spans(&self, source: &str) -> Vec<(String, usize, usize, usize)> {
        let tree = parse(source);
        let mut out: Vec<(String, usize, usize, usize)> = Vec::new();
        for node in descendants(tree.root_node()) {
            if node.kind() != "use_declaration" {
                continue;
            }
            let Some(arg) = node.child_by_field_name("argument") else {
                continue;
            };
            for (root, relative) in use_crate_root_nodes(arg) {
                if relative {
                    continue;
                }
                let text = node_text(root, source);
                if text.is_empty() {
                    continue;
                }
                let line = root.start_position().row + 1;
                let col_start = root.start_position().column;
                let col_end = col_start + text.len();
                out.push((text.to_string(), line, col_start, col_end));
            }
        }
        out.sort_by(|a, b| (a.1, a.2, &a.0).cmp(&(b.1, b.2, &b.0)));
        out
    }

    /// Names bound by repo-relative imports (`use crate::x::Y` binds `Y`;
    /// `use super::{a, b}` binds `a`, `b`). Repo-internal neighbours are not
    /// foreign voice.
    pub fn internal_import_bindings(&self, source: &str) -> HashSet<String> {
        let tree = parse(source);
        let mut out = HashSet::new();
        for node in descendants(tree.root_node()) {
            if node.kind() != "use_declaration" {
                continue;
            }
            let Some(arg) = node.child_by_field_name("argument") else {
                continue;
            };
            let roots = use_crate_root_nodes(arg);
            if roots.iter().any(|(_, relative)| *relative) {
                for name in use_bound_names(arg, source) {
                    out.insert(name);
                }
            }
        }
        out
    }

    /// 1-indexed line numbers that carry prose — line (`//`, `///`, `//!`) and
    /// block (`/* … */`, `/** … */`) comments.
    pub fn prose_line_ranges(&self, source: &str) -> HashSet<usize> {
        let tree = parse(source);
        let mut rows: HashSet<usize> = HashSet::new();
        for node in descendants(tree.root_node()) {
            if matches!(node.kind(), "line_comment" | "block_comment") {
                for r in (node.start_position().row + 1)..=(node.end_position().row + 1) {
                    rows.insert(r);
                }
            }
        }
        rows
    }

    /// True if the file is overwhelmingly top-level data literals (`const`/
    /// `static` array tables bound at module scope).
    pub fn is_data_dominant(&self, source: &str, threshold: f64) -> bool {
        if source.trim().is_empty() {
            return false;
        }
        let tree = parse(source);
        let total_nonblank = source.lines().filter(|l| !l.trim().is_empty()).count();
        if total_nonblank == 0 {
            return false;
        }
        let mut data_rows: HashSet<usize> = HashSet::new();
        collect_rust_data_rows(tree.root_node(), &mut data_rows);
        (data_rows.len() as f64) / (total_nonblank as f64) > threshold
    }

    /// 1-indexed line numbers covered by top-level data-literal declarations.
    pub fn data_literal_lines(&self, source: &str) -> HashSet<usize> {
        if source.trim().is_empty() {
            return HashSet::new();
        }
        let tree = parse(source);
        let mut rows: HashSet<usize> = HashSet::new();
        collect_rust_data_rows(tree.root_node(), &mut rows);
        rows.into_iter().map(|r| r + 1).collect()
    }

    /// Names the source binds to callable definitions — `fn` items, `fn`
    /// signatures (trait methods), and `let name = |…| …` closure bindings.
    /// Local-binding attestation: code calling what it defines is not foreign
    /// voice.
    pub fn callable_definitions(&self, source: &str) -> HashSet<String> {
        let tree = parse(source);
        let mut out = HashSet::new();
        for node in descendants(tree.root_node()) {
            match node.kind() {
                "function_item" | "function_signature_item" => {
                    if let Some(name) = node.child_by_field_name("name") {
                        out.insert(node_text(name, source).to_string());
                    }
                }
                // Type declarations: their names lead `Type::variant` /
                // `Type::method` calls, so calling a repo-declared type is
                // internal cross-file code, not a foreign namespace.
                "struct_item" | "enum_item" | "union_item" | "type_item" | "trait_item" => {
                    if let Some(name) = node.child_by_field_name("name") {
                        out.insert(node_text(name, source).to_string());
                    }
                }
                "let_declaration" => {
                    if let (Some(pattern), Some(value)) = (
                        node.child_by_field_name("pattern"),
                        node.child_by_field_name("value"),
                    ) {
                        if pattern.kind() == "identifier" && value.kind() == "closure_expression" {
                            out.insert(node_text(pattern, source).to_string());
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// Every locally bound value name — `let` pattern targets and function
    /// parameters. Attests bare calls: calling a value you just bound is
    /// neighbourhood behaviour, not foreign voice.
    pub fn value_bindings(&self, source: &str) -> HashSet<String> {
        let tree = parse(source);
        let mut out = HashSet::new();
        for node in descendants(tree.root_node()) {
            match node.kind() {
                "let_declaration" => {
                    if let Some(pattern) = node.child_by_field_name("pattern") {
                        collect_pattern_ids(pattern, source, &mut out);
                    }
                }
                "parameter" => {
                    if let Some(pattern) = node.child_by_field_name("pattern") {
                        collect_pattern_ids(pattern, source, &mut out);
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// True if the header carries a generated-file marker (`// @generated`, or
    /// a `// Code generated … DO NOT EDIT.` line) within the head
    /// `HEADER_LINE_LIMIT` lines.
    pub fn is_auto_generated(&self, source: &str, _markers: &[String]) -> bool {
        if source.is_empty() {
            return false;
        }
        let tree = parse(source);
        for node in descendants(tree.root_node()) {
            if !matches!(node.kind(), "line_comment" | "block_comment") {
                continue;
            }
            if node.start_position().row >= HEADER_LINE_LIMIT {
                continue;
            }
            let text = node_text(node, source).trim();
            if text.contains("@generated") {
                return true;
            }
            if text.starts_with("// Code generated") && text.contains("DO NOT EDIT") {
                return true;
            }
        }
        false
    }

    /// 1-indexed inclusive `(start_line, end_line)` spans for top-level `fn`
    /// items and `impl` methods. Empty when the file failed to parse.
    pub fn enumerate_sampleable_ranges(&self, source: &str) -> Vec<(usize, usize)> {
        let tree = parse(source);
        let root = tree.root_node();
        if root.has_error() {
            return Vec::new();
        }
        let mut ranges = Vec::new();
        for child in children(root) {
            match child.kind() {
                "function_item" => {
                    ranges.push((child.start_position().row + 1, child.end_position().row + 1));
                }
                "impl_item" => {
                    if let Some(body) = child.child_by_field_name("body") {
                        for method in named_children(body) {
                            if method.kind() == "function_item" {
                                ranges.push((
                                    method.start_position().row + 1,
                                    method.end_position().row + 1,
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        ranges
    }

    /// Rust keywords, reserved words, and common std builtins — noise tokens
    /// that would dominate `common here:` without saying anything.
    pub fn identifier_noise(&self) -> &HashSet<String> {
        &self.noise
    }
}

/// Rust keywords + common std type/macro identifiers.
const NOISE: &[&str] = &[
    "fn", "let", "mut", "impl", "trait", "struct", "enum", "match", "if", "else", "for", "while",
    "loop", "return", "use", "pub", "mod", "where", "async", "await", "move", "ref", "self",
    "Self", "super", "crate", "dyn", "as", "const", "static", "unsafe", "true", "false", "Some",
    "None", "Ok", "Err", "vec", "println", "format", "Box", "Vec", "String", "Option", "Result",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_are_crate_roots() {
        let adapter = RustAdapter::new();
        let src = "use serde::Serialize;\nuse std::collections::HashMap;\n";
        let imports = adapter.extract_imports(src);
        assert!(imports.contains("serde"));
        assert!(imports.contains("std"));
        // Crate root only, not the leaf.
        assert!(!imports.contains("Serialize"));
        assert!(!imports.contains("HashMap"));
    }

    #[test]
    fn use_list_shares_one_crate_root() {
        let adapter = RustAdapter::new();
        let src = "use tokio::{spawn, select};\n";
        let imports = adapter.extract_imports(src);
        assert!(imports.contains("tokio"));
        assert_eq!(imports.len(), 1);
    }

    #[test]
    fn relative_use_is_not_a_foreign_import() {
        let adapter = RustAdapter::new();
        let src = "use crate::foo::Bar;\nuse super::baz;\nuse self::inner::Thing;\n";
        let imports = adapter.extract_imports(src);
        assert!(imports.is_empty());
        // …it lands in internal bindings instead.
        let internal = adapter.internal_import_bindings(src);
        assert!(internal.contains("Bar"));
        assert!(internal.contains("baz"));
        assert!(internal.contains("Thing"));
    }

    #[test]
    fn callable_definitions_cover_fns_and_closure_bindings() {
        let adapter = RustAdapter::new();
        let src = "fn foo() {}\n\nstruct S;\nimpl S {\n    fn bar(&self) {}\n}\n\nfn make() {\n    let baz = |x: i32| x + 1;\n}\n";
        let defs = adapter.callable_definitions(src);
        assert!(defs.contains("foo"));
        assert!(defs.contains("bar"));
        assert!(defs.contains("baz"));
    }

    #[test]
    fn value_bindings_cover_let_and_params() {
        let adapter = RustAdapter::new();
        let src = "fn f(a: i32, b: String) {\n    let x = a;\n    let (y, z) = (1, 2);\n}\n";
        let binds = adapter.value_bindings(src);
        assert!(binds.contains("a"));
        assert!(binds.contains("b"));
        assert!(binds.contains("x"));
        assert!(binds.contains("y"));
        assert!(binds.contains("z"));
    }

    #[test]
    fn sampleable_ranges_cover_top_level_fns_and_impl_methods() {
        let adapter = RustAdapter::new();
        let src = "fn a() {\n    let _ = 1;\n}\n\nstruct S;\nimpl S {\n    fn b(&self) {\n        let _ = 2;\n    }\n}\n";
        let ranges = adapter.enumerate_sampleable_ranges(src);
        // top-level `a` plus method `b`.
        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn generated_header_is_detected() {
        let adapter = RustAdapter::new();
        let gen = "// @generated by prost-build\n\npub struct M {}\n";
        assert!(adapter.is_auto_generated(gen, &crate::config::default_generated_markers()));
        let marker = "// Code generated by tonic. DO NOT EDIT.\n\npub fn f() {}\n";
        assert!(adapter.is_auto_generated(marker, &crate::config::default_generated_markers()));
        let hand = "// A normal comment.\n\npub fn f() {}\n";
        assert!(!adapter.is_auto_generated(hand, &crate::config::default_generated_markers()));
    }

    #[test]
    fn top_level_data_tables_are_data_dominant() {
        let adapter = RustAdapter::new();
        let src = "static CITIES: [&str; 5] = [\n    \"a\",\n    \"b\",\n    \"c\",\n    \"d\",\n    \"e\",\n];\n";
        assert!(adapter.is_data_dominant(src, 0.65));
        assert!(!adapter.data_literal_lines(src).is_empty());
    }

    #[test]
    fn code_is_not_data_dominant() {
        let adapter = RustAdapter::new();
        let src = "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
        assert!(!adapter.is_data_dominant(src, 0.65));
    }

    #[test]
    fn prose_ranges_cover_line_and_block_comments() {
        let adapter = RustAdapter::new();
        let src = "// line\nfn f() {}\n/* block */\n";
        let rows = adapter.prose_line_ranges(src);
        assert!(rows.contains(&1));
        assert!(rows.contains(&3));
    }

    #[test]
    fn identifier_noise_contains_keywords_and_builtins() {
        let adapter = RustAdapter::new();
        assert!(adapter.identifier_noise().contains("fn"));
        assert!(adapter.identifier_noise().contains("impl"));
        assert!(adapter.identifier_noise().contains("Vec"));
        assert_eq!(adapter.identifier_noise().len(), NOISE.len());
    }
}
