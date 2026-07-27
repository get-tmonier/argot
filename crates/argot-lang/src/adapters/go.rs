//! Go language adapter — the first language port beyond Python / TypeScript
//! and the reference for the series.
//!
//! Structurally a sibling of `python.rs` / `typescript.rs`: manual
//! `TreeCursor` traversal over the pinned tree-sitter-go grammar, adapted to
//! Go's node kinds. Notes worth keeping in mind:
//! - Imports are quoted paths (`import "github.com/x/y"`); the module string is
//!   the whole path, not a top-level segment (Go paths have no `a.b.c` splitting
//!   the way Python dotted modules do).
//! - Go has no relative-import concept, so `internal_import_bindings` is empty —
//!   repo-internal neighbours are discovered via `extract_imports` at fit time.
//! - Data-dominance is computed against the Go grammar directly (the shared
//!   `filters::data_dominant` is Python-grammar-bound), mirroring how the
//!   TypeScript adapter rolls its own.

use std::collections::HashSet;
use std::path::Path;

use tree_sitter::{Node, Tree};

use super::Language;

use super::HEADER_LINE_LIMIT;

/// Ratio of data-literal rows above which a file is data-dominant.
/// Fraction of a composite literal's elements that must be value literals for
/// it to count as static data.
const VALUE_DOMINANT_THRESHOLD: f64 = 0.8;

/// Node kinds that count as static value literals inside a composite literal.
const GO_VALUE_LITERAL_TYPES: &[&str] = &[
    "int_literal",
    "float_literal",
    "imaginary_literal",
    "rune_literal",
    "interpreted_string_literal",
    "raw_string_literal",
    "true",
    "false",
    "nil",
    "iota",
    "composite_literal",
    "literal_value",
];

/// Parse Go `source` into a tree-sitter `Tree` (reused per-thread parser).
pub(crate) fn parse(source: &str) -> Tree {
    crate::ts_parse::parse(source, Language::Go)
        .expect("tree-sitter parse never returns None without a timeout")
}

/// Collect every descendant of `node` (pre-order, excluding `node` itself).
fn descendants<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    fn go<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            out.push(child);
            go(child, out);
        }
    }
    for child in node.children(&mut cursor) {
        out.push(child);
        go(child, &mut out);
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

/// Direct children of `node` whose field name is `field` and kind is `kind`.
///
/// A `var`/`const`/parameter spec can carry the same field on several children
/// (`var a, b int` has two `name` fields), which `child_by_field_name` cannot
/// express — hence the manual cursor walk.
fn field_children_of_kind<'a>(node: Node<'a>, field: &str, kind: &str) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            if cursor.field_name() == Some(field) && cursor.node().kind() == kind {
                out.push(cursor.node());
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    out
}

fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

/// Strip the surrounding quotes from a Go string-literal node text
/// (`"fmt"` → `fmt`, `` `raw` `` → `raw`).
fn strip_quotes(text: &str) -> &str {
    let bytes = text.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' || first == b'`') && last == first {
            return &text[1..text.len() - 1];
        }
    }
    text
}

/// The value node of a composite-literal element (unwrapping `keyed_element`s
/// to their value side), or `None`.
fn go_element_value(el: Node) -> Option<Node> {
    match el.kind() {
        "literal_element" => el.named_child(0),
        "keyed_element" => el
            .child_by_field_name("value")
            .and_then(|v| v.named_child(0)),
        _ => None,
    }
}

/// True if ≥80% of a composite literal's immediate elements are value literals.
/// Empty / bodyless literals return true (conservative — allow).
fn is_go_value_literal_dominant(comp: Node) -> bool {
    let Some(body) = comp.child_by_field_name("body") else {
        return true;
    };
    let values: Vec<Node> = children(body)
        .into_iter()
        .filter(|c| matches!(c.kind(), "literal_element" | "keyed_element"))
        .filter_map(go_element_value)
        .collect();
    if values.is_empty() {
        return true;
    }
    let literal_count = values
        .iter()
        .filter(|v| GO_VALUE_LITERAL_TYPES.contains(&v.kind()))
        .count();
    (literal_count as f64) / (values.len() as f64) >= VALUE_DOMINANT_THRESHOLD
}

/// The `var_spec`/`const_spec` nodes under a top-level `var`/`const`
/// declaration (unwrapping the grouped `var ( … )` `var_spec_list`).
fn declaration_specs<'a>(decl: Node<'a>) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    for sub in children(decl) {
        match sub.kind() {
            "var_spec" | "const_spec" => out.push(sub),
            "var_spec_list" => {
                for s in children(sub) {
                    if s.kind() == "var_spec" {
                        out.push(s);
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Add 0-indexed row spans of top-level data-literal `var`/`const`
/// declarations to `rows`.
fn collect_go_data_rows(root: Node, rows: &mut HashSet<usize>) {
    for child in children(root) {
        if !matches!(child.kind(), "var_declaration" | "const_declaration") {
            continue;
        }
        for spec in declaration_specs(child) {
            let Some(value) = spec.child_by_field_name("value") else {
                continue;
            };
            for v in named_children(value) {
                if v.kind() == "composite_literal" && is_go_value_literal_dominant(v) {
                    for r in spec.start_position().row..=spec.end_position().row {
                        rows.insert(r);
                    }
                }
            }
        }
    }
}

/// `GoAdapter` — the language adapter for `.go` sources.
#[derive(Debug, Default)]
pub struct GoAdapter {
    noise: HashSet<String>,
}

/// Fold a Go module major-version suffix (`/v2`, `/v3`, …) out of an import
/// path so a version bump (`x/y/bson` → `x/y/v2/bson`) is not read as a
/// brand-new dependency (issue: a `/vN` bump flagged foreign). Only pure `vN`
/// segments with N ≥ 2 are dropped — Go's major-version convention. `v1` and
/// API-version segments like `k8s.io/api/autoscaling/v1` carry no `/vN` suffix
/// (v1 modules have none), so a genuinely new API surface (`.../v2`) still
/// folds to a different key than the attested `.../v1` and stays flagged.
fn fold_go_major_version(spec: &str) -> String {
    if !spec.contains("/v") {
        return spec.to_string();
    }
    let kept: Vec<&str> = spec
        .split('/')
        .filter(|seg| !is_major_version(seg))
        .collect();
    kept.join("/")
}

/// A pure `vN` path segment with N ≥ 2 (Go major-version marker). `v1`, `v0`,
/// and mixed segments like `v2beta1` are not markers.
fn is_major_version(seg: &str) -> bool {
    seg.strip_prefix('v')
        .filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|n| n.parse::<u32>().ok())
        .is_some_and(|n| n >= 2)
}

impl GoAdapter {
    pub fn new() -> Self {
        Self {
            noise: NOISE.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Module paths imported in `source` (`"fmt"`, `"github.com/x/y"`), with any
    /// Go major-version suffix folded out (`fold_go_major_version`) so v1 and v2
    /// of the same module share a key.
    pub fn extract_imports(&self, source: &str) -> HashSet<String> {
        let tree = parse(source);
        let mut out = HashSet::new();
        for node in descendants(tree.root_node()) {
            if node.kind() != "import_spec" {
                continue;
            }
            if let Some(path) = node.child_by_field_name("path") {
                let spec = strip_quotes(node_text(path, source));
                if !spec.is_empty() {
                    out.insert(fold_go_major_version(spec));
                }
            }
        }
        out
    }

    /// Repo-owned module paths from `go.mod` files: importing any package
    /// under the repo's own module path is never foreign voice, including
    /// packages that did not exist at fit time (issue #92 — new internal
    /// packages flooded the import tripwire).
    pub fn resolve_repo_modules(&self, repo_root: &Path) -> super::RepoModules {
        let mut modules = super::RepoModules::default();
        for gomod in super::walk_files_with_suffix(repo_root, &["go.mod"]) {
            let Ok(text) = crate::text::read_text_lossy(&gomod) else {
                continue;
            };
            for line in text.lines() {
                if let Some(rest) = line.trim().strip_prefix("module ") {
                    let raw = rest.trim().trim_matches('"');
                    if !raw.is_empty() {
                        // Fold `/vN` so imports normalized by `extract_imports`
                        // match the repo's own module the same way.
                        let path = fold_go_major_version(raw);
                        modules.prefixes.insert(format!("{path}/"));
                        modules.exact.insert(path);
                    }
                    break;
                }
            }
        }
        modules
    }

    /// Like `extract_imports` but each path carries its `(spec, line,
    /// col_start, col_end)` span. Line 1-indexed; columns 0-indexed byte
    /// offsets covering the path text (opening quote skipped), end exclusive.
    /// Sorted by `(line, col_start, spec)`.
    pub fn extract_imports_with_spans(&self, source: &str) -> Vec<(String, usize, usize, usize)> {
        let tree = parse(source);
        let mut out: Vec<(String, usize, usize, usize)> = Vec::new();
        for node in descendants(tree.root_node()) {
            if node.kind() != "import_spec" {
                continue;
            }
            if let Some(path) = node.child_by_field_name("path") {
                let spec = strip_quotes(node_text(path, source));
                if spec.is_empty() {
                    continue;
                }
                let line = path.start_position().row + 1;
                // Skip the opening quote so the underline lands on the path.
                let col_start = path.start_position().column + 1;
                // Columns cover the original path text (caret lands on the real
                // `/v2/…`); the key is folded to match `extract_imports`.
                let col_end = col_start + spec.len();
                out.push((fold_go_major_version(spec), line, col_start, col_end));
            }
        }
        out.sort_by(|a, b| (a.1, a.2, &a.0).cmp(&(b.1, b.2, &b.0)));
        out
    }

    /// Repo-internal import bindings. Go has no relative-import syntax, so this
    /// is always empty (repo-internal modules are discovered via
    /// `extract_imports` at fit time).
    pub fn internal_import_bindings(&self, _source: &str) -> HashSet<String> {
        HashSet::new()
    }

    /// 1-indexed line numbers that carry prose — line (`//`) and block
    /// (`/* … */`) comments.
    fn prose_mask(&self, source: &str) -> crate::ts_parse::ProseMask {
        let tree = parse(source);
        let mut mask = crate::ts_parse::ProseMask::default();
        for node in descendants(tree.root_node()) {
            if node.kind() == "comment" {
                mask.add(source, node);
            }
        }
        mask
    }

    pub fn prose_line_ranges(&self, source: &str) -> HashSet<usize> {
        self.prose_mask(source).rows
    }

    /// Prose sharing a line with code: blanked in place, so the code survives
    /// and the words do not reach the scorers.
    pub fn prose_spans(&self, source: &str) -> Vec<(usize, usize, usize)> {
        self.prose_mask(source).spans
    }

    /// True if the file is overwhelmingly top-level data literals (slice / map /
    /// array / struct composite literals bound at package scope).
    pub fn is_data_dominant(&self, source: &str, threshold: f64) -> bool {
        if source.trim().is_empty() {
            return false;
        }
        let tree = parse(source);
        let root = tree.root_node();
        let total_nonblank = source.lines().filter(|l| !l.trim().is_empty()).count();
        if total_nonblank == 0 {
            return false;
        }
        let mut data_rows: HashSet<usize> = HashSet::new();
        collect_go_data_rows(root, &mut data_rows);
        (data_rows.len() as f64) / (total_nonblank as f64) > threshold
    }

    /// 1-indexed line numbers covered by top-level data-literal declarations.
    pub fn data_literal_lines(&self, source: &str) -> HashSet<usize> {
        if source.trim().is_empty() {
            return HashSet::new();
        }
        let tree = parse(source);
        let mut rows: HashSet<usize> = HashSet::new();
        collect_go_data_rows(tree.root_node(), &mut rows);
        rows.into_iter().map(|r| r + 1).collect()
    }

    /// Names the source binds to callable definitions — `func`/method
    /// declarations, and `var`/`const`/`:=` bindings to a `func` literal.
    /// Local-binding attestation: code calling what it defines is not foreign
    /// voice.
    /// Function/method definitions with their line ranges — the embeddable units
    /// for the semantic index. Top-level `func` declarations and methods (`func
    /// (r Recv) Name(...)`). Function literals bound to vars are left to base
    /// callee attestation, not embedded as standalone units.
    pub fn callable_bodies(&self, source: &str) -> Vec<super::CallableBody> {
        let tree = parse(source);
        let mut out = Vec::new();
        for node in descendants(tree.root_node()) {
            if matches!(node.kind(), "function_declaration" | "method_declaration") {
                if let Some(name) = node.child_by_field_name("name") {
                    out.push(super::CallableBody {
                        symbol: node_text(name, source).to_string(),
                        start_line: node.start_position().row + 1,
                        end_line: node.end_position().row + 1,
                    });
                }
            }
        }
        out
    }

    pub fn callable_definitions(&self, source: &str) -> HashSet<String> {
        let tree = parse(source);
        let mut out = HashSet::new();
        for node in descendants(tree.root_node()) {
            match node.kind() {
                "function_declaration" | "method_declaration" => {
                    if let Some(name) = node.child_by_field_name("name") {
                        out.insert(node_text(name, source).to_string());
                    }
                }
                "short_var_declaration" => {
                    if let (Some(left), Some(right)) = (
                        node.child_by_field_name("left"),
                        node.child_by_field_name("right"),
                    ) {
                        bind_func_literals(
                            &named_children(left),
                            &named_children(right),
                            source,
                            &mut out,
                        );
                    }
                }
                "var_spec" | "const_spec" => {
                    let names = field_children_of_kind(node, "name", "identifier");
                    if let Some(value) = node.child_by_field_name("value") {
                        bind_func_literals(&names, &named_children(value), source, &mut out);
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// Every locally bound value name — `:=` / `var` / `const` targets and
    /// function parameters. Attests bare calls: calling a value you just bound
    /// is neighbourhood behaviour, not foreign voice.
    pub fn value_bindings(&self, source: &str) -> HashSet<String> {
        let tree = parse(source);
        let mut out = HashSet::new();
        for node in descendants(tree.root_node()) {
            match node.kind() {
                "short_var_declaration" => {
                    if let Some(left) = node.child_by_field_name("left") {
                        for id in named_children(left) {
                            if id.kind() == "identifier" {
                                out.insert(node_text(id, source).to_string());
                            }
                        }
                    }
                }
                "var_spec" | "const_spec" => {
                    for name in field_children_of_kind(node, "name", "identifier") {
                        out.insert(node_text(name, source).to_string());
                    }
                }
                "parameter_declaration" => {
                    for name in field_children_of_kind(node, "name", "identifier") {
                        out.insert(node_text(name, source).to_string());
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// True if the header carries Go's generated-file marker
    /// (`// Code generated … DO NOT EDIT.`), within the head `HEADER_LINE_LIMIT`
    /// lines.
    pub fn is_auto_generated(&self, source: &str, _markers: &[String]) -> bool {
        if source.is_empty() {
            return false;
        }
        let tree = parse(source);
        for node in descendants(tree.root_node()) {
            if node.kind() != "comment" {
                continue;
            }
            if node.start_position().row >= HEADER_LINE_LIMIT {
                continue;
            }
            let text = node_text(node, source).trim();
            if text.starts_with("// Code generated") && text.contains("DO NOT EDIT") {
                return true;
            }
        }
        false
    }

    /// 1-indexed inclusive `(start_line, end_line)` spans for top-level
    /// function / method declarations. Empty when the file failed to parse.
    pub fn enumerate_sampleable_ranges(&self, source: &str) -> Vec<(usize, usize)> {
        let tree = parse(source);
        let root = tree.root_node();
        if root.has_error() {
            return Vec::new();
        }
        let mut ranges = Vec::new();
        for child in children(root) {
            if matches!(child.kind(), "function_declaration" | "method_declaration") {
                ranges.push((child.start_position().row + 1, child.end_position().row + 1));
            }
        }
        ranges
    }

    /// Go keywords, predeclared constants, and common builtins — noise tokens
    /// that would dominate `common here:` without saying anything.
    pub fn identifier_noise(&self) -> &HashSet<String> {
        &self.noise
    }
}

/// Bind each name whose positionally-matched value is a `func_literal`.
fn bind_func_literals(names: &[Node], values: &[Node], source: &str, out: &mut HashSet<String>) {
    for (name, value) in names.iter().zip(values.iter()) {
        if value.kind() == "func_literal" && name.kind() == "identifier" {
            out.insert(node_text(*name, source).to_string());
        }
    }
}

/// Go keywords + predeclared identifiers + common builtins.
const NOISE: &[&str] = &[
    "func",
    "package",
    "import",
    "return",
    "if",
    "else",
    "for",
    "range",
    "var",
    "const",
    "type",
    "struct",
    "interface",
    "map",
    "chan",
    "go",
    "defer",
    "select",
    "switch",
    "case",
    "default",
    "break",
    "continue",
    "fallthrough",
    "nil",
    "true",
    "false",
    "iota",
    "error",
    "make",
    "len",
    "cap",
    "append",
    "panic",
    "recover",
];

#[cfg(test)]
mod tests;
