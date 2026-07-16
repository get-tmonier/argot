//! Data-dominant file detector.
//!
//! A structural, I/O-free heuristic: parse with tree-sitter and walk
//! module-level statements plus top-level class bodies. Any `assignment`
//! whose right-hand side is a `list`/`tuple`/`dictionary`/`set` literal made
//! mostly of value literals contributes its line span. If that span covers
//! more than `threshold` of the non-blank lines, the file is data-dominant.

use std::collections::HashSet;
use tree_sitter::{Node, Parser, Tree};

use crate::text::splitlines;

/// RHS node types that count as "static data literals".
const DATA_LITERAL_TYPES: &[&str] = &["list", "tuple", "dictionary", "set"];

/// Named-value node types that count as literal data.
const VALUE_LITERAL_TYPES: &[&str] = &[
    "string",
    "concatenated_string",
    "integer",
    "float",
    "true",
    "false",
    "none",
    "list",
    "tuple",
    "dictionary",
    "set",
];

const VALUE_DOMINANT_THRESHOLD: f64 = 0.8;

fn parse(source: &str) -> Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .expect("tree-sitter python grammar loads");
    parser
        .parse(source.as_bytes(), None)
        .expect("tree-sitter parse never returns None without a timeout")
}

fn children<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).collect()
}

/// True if ≥80% of the node's immediate named values are literal data.
/// Empty containers and unknown types return true (conservative — allow).
fn is_value_literal_dominant(node: Node) -> bool {
    let values: Vec<Node> = match node.kind() {
        "list" | "tuple" | "set" => children(node)
            .into_iter()
            .filter(|c| c.is_named())
            .collect(),
        "dictionary" => {
            let mut vals = Vec::new();
            for child in children(node) {
                match child.kind() {
                    "pair" => {
                        if let Some(val) = child.child_by_field_name("value") {
                            vals.push(val);
                        }
                    }
                    // splat (**x) is non-literal
                    "dictionary_splat" => vals.push(child),
                    _ => {}
                }
            }
            vals
        }
        _ => return true,
    };
    if values.is_empty() {
        return true;
    }
    let literal_count = values
        .iter()
        .filter(|v| VALUE_LITERAL_TYPES.contains(&v.kind()))
        .count();
    (literal_count as f64) / (values.len() as f64) >= VALUE_DOMINANT_THRESHOLD
}

/// Return the right-hand-side child of an assignment node — the first child
/// after the `=` token — or `None`.
fn get_rhs(node: Node) -> Option<Node> {
    let mut found_eq = false;
    for child in children(node) {
        if found_eq {
            return Some(child);
        }
        if child.kind() == "=" {
            found_eq = true;
        }
    }
    None
}

/// Scan sibling statement nodes, adding data-literal spans (0-indexed rows).
fn collect_stmt_data_rows(stmts: &[Node], rows: &mut HashSet<usize>) {
    for stmt in stmts {
        let target = if stmt.kind() == "assignment" {
            Some(*stmt)
        } else if stmt.kind() == "expression_statement" {
            children(*stmt)
                .into_iter()
                .find(|c| c.kind() == "assignment")
        } else {
            None
        };

        let Some(target) = target else { continue };

        if let Some(rhs) = get_rhs(target) {
            if DATA_LITERAL_TYPES.contains(&rhs.kind()) && is_value_literal_dominant(rhs) {
                for r in stmt.start_position().row..=stmt.end_position().row {
                    rows.insert(r);
                }
            }
        }
    }
}

/// 0-indexed row numbers covered by data-literal assignments, scanning both
/// module-level statements and top-level class bodies.
fn extract_data_literal_lines(root: Node) -> HashSet<usize> {
    let mut rows = HashSet::new();
    let top = children(root);
    collect_stmt_data_rows(&top, &mut rows);

    for child in &top {
        if child.kind() == "class_definition" {
            for sub in children(*child) {
                if sub.kind() == "block" {
                    collect_stmt_data_rows(&children(sub), &mut rows);
                }
            }
        }
    }
    rows
}

/// 1-indexed line numbers covered by data-literal assignment spans — the
/// row-granular view behind [`is_data_dominant`]. Empty on parse failure.
pub fn data_literal_lines(source: &str) -> HashSet<usize> {
    if source.is_empty() || source.trim().is_empty() {
        return HashSet::new();
    }
    let tree = parse(source);
    let root = tree.root_node();
    if root.has_error() && root.kind() == "ERROR" {
        return HashSet::new();
    }
    extract_data_literal_lines(root)
        .into_iter()
        .map(|r| r + 1)
        .collect()
}

/// True if the file is overwhelmingly top-level data-literal assignments.
pub fn is_data_dominant(source: &str, threshold: f64) -> bool {
    if source.is_empty() || source.trim().is_empty() {
        return false;
    }

    let tree = parse(source);
    let root = tree.root_node();

    // Tolerate minor errors, but bail if the whole tree is an ERROR node.
    if root.has_error() && root.kind() == "ERROR" {
        return false;
    }

    let total_nonblank = splitlines(source)
        .iter()
        .filter(|line| !line.trim().is_empty())
        .count();
    if total_nonblank == 0 {
        return false;
    }

    let data_rows = extract_data_literal_lines(root);
    let ratio = (data_rows.len() as f64) / (total_nonblank as f64);
    ratio > threshold
}

#[cfg(test)]
mod tests;
