//! Typicality filter.
//!
//! AST-derived structural predicate. Drops structurally atypical hunks (data
//! tables, generated boilerplate) from both the calibration pool and
//! inference-time control scoring. Parses source directly with the pinned
//! tree-sitter Python / TypeScript grammars and walks the tree manually
//! (matching the style in `tokenize.rs` and `scoring/adapters/python.rs`), so
//! the five features and two verdicts match the golden reference exactly.

use std::collections::{HashMap, HashSet};

use tree_sitter::Node;

use crate::scoring::adapters::Language;

// Node types per language. Values are tree-sitter grammar node-type strings,
// listed verbatim.
const PY_LITERAL_NODE_TYPES: &[&str] = &[
    "string",
    "concatenated_string",
    "integer",
    "float",
    "true",
    "false",
    "none",
];

const PY_CONTROL_NODE_TYPES: &[&str] = &[
    "if_statement",
    "for_statement",
    "while_statement",
    "try_statement",
    "return_statement",
    "function_definition",
    "async_function_definition",
    "class_definition",
    "with_statement",
    "raise_statement",
];

const TS_LITERAL_NODE_TYPES: &[&str] = &[
    "string",
    "template_string",
    "number",
    "true",
    "false",
    "null",
    "undefined",
    "regex",
];

const TS_CONTROL_NODE_TYPES: &[&str] = &[
    "if_statement",
    "for_statement",
    "for_in_statement",
    "for_of_statement",
    "while_statement",
    "do_statement",
    "try_statement",
    "return_statement",
    "function_declaration",
    "function_expression",
    "arrow_function",
    "class_declaration",
    "method_definition",
    "throw_statement",
    "switch_statement",
];

// Go node-type sets — the direct analog of the Python/TS sets against the Go
// grammar (same absolute cutoffs; no Go-specific tuning).
const GO_LITERAL_NODE_TYPES: &[&str] = &[
    "int_literal",
    "float_literal",
    "imaginary_literal",
    "rune_literal",
    "interpreted_string_literal",
    "raw_string_literal",
    "true",
    "false",
    "nil",
];

const GO_CONTROL_NODE_TYPES: &[&str] = &[
    "if_statement",
    "for_statement",
    "expression_switch_statement",
    "type_switch_statement",
    "select_statement",
    "return_statement",
    "function_declaration",
    "method_declaration",
    "defer_statement",
    "go_statement",
];

// Rust node-type sets — the direct analog of the Python/TS/Go sets against the
// Rust grammar (same absolute cutoffs; no Rust-specific tuning). Rust literals
// and control flow are expressions, so the kinds are the `*_expression` /
// `*_item` analogs.
const RUST_LITERAL_NODE_TYPES: &[&str] = &[
    "integer_literal",
    "float_literal",
    "string_literal",
    "raw_string_literal",
    "char_literal",
    "boolean_literal",
    "negative_literal",
];

const RUST_CONTROL_NODE_TYPES: &[&str] = &[
    "if_expression",
    "for_expression",
    "while_expression",
    "loop_expression",
    "match_expression",
    "return_expression",
    "function_item",
    "closure_expression",
];

const C_LITERAL_NODE_TYPES: &[&str] = &[
    "number_literal",
    "string_literal",
    "char_literal",
    "concatenated_string",
    "true",
    "false",
    "null",
];

const C_CONTROL_NODE_TYPES: &[&str] = &[
    "if_statement",
    "for_statement",
    "while_statement",
    "do_statement",
    "switch_statement",
    "return_statement",
    "function_definition",
    "goto_statement",
];

const JAVA_LITERAL_NODE_TYPES: &[&str] = &[
    "string_literal",
    "character_literal",
    "decimal_integer_literal",
    "hex_integer_literal",
    "octal_integer_literal",
    "binary_integer_literal",
    "decimal_floating_point_literal",
    "hex_floating_point_literal",
    "true",
    "false",
    "null_literal",
];

const JAVA_CONTROL_NODE_TYPES: &[&str] = &[
    "if_statement",
    "for_statement",
    "enhanced_for_statement",
    "while_statement",
    "do_statement",
    "try_statement",
    "try_with_resources_statement",
    "return_statement",
    "method_declaration",
    "constructor_declaration",
    "class_declaration",
    "switch_expression",
    "throw_statement",
    "synchronized_statement",
];

const CS_LITERAL_NODE_TYPES: &[&str] = &[
    "integer_literal",
    "real_literal",
    "string_literal",
    "verbatim_string_literal",
    "raw_string_literal",
    "interpolated_string_expression",
    "character_literal",
    "boolean_literal",
    "null_literal",
];

const CS_CONTROL_NODE_TYPES: &[&str] = &[
    "if_statement",
    "for_statement",
    "for_each_statement",
    "while_statement",
    "do_statement",
    "switch_statement",
    "try_statement",
    "return_statement",
    "throw_statement",
    "method_declaration",
    "constructor_declaration",
    "local_function_statement",
    "class_declaration",
    "using_statement",
    "lock_statement",
];

const PHP_LITERAL_NODE_TYPES: &[&str] = &[
    "string",
    "encapsed_string",
    "heredoc",
    "nowdoc",
    "integer",
    "float",
    "boolean",
    "null",
];

const PHP_CONTROL_NODE_TYPES: &[&str] = &[
    "if_statement",
    "for_statement",
    "foreach_statement",
    "while_statement",
    "do_statement",
    "try_statement",
    "switch_statement",
    "match_expression",
    "return_statement",
    "throw_expression",
    "function_definition",
    "method_declaration",
    "class_declaration",
    "interface_declaration",
    "trait_declaration",
    "enum_declaration",
    "anonymous_function",
    "arrow_function",
];

const CPP_LITERAL_NODE_TYPES: &[&str] = &[
    "string_literal",
    "raw_string_literal",
    "concatenated_string",
    "number_literal",
    "char_literal",
    "true",
    "false",
    "null",
    "nullptr",
    "user_defined_literal",
];

const CPP_CONTROL_NODE_TYPES: &[&str] = &[
    "if_statement",
    "for_statement",
    "for_range_loop",
    "while_statement",
    "do_statement",
    "try_statement",
    "return_statement",
    "function_definition",
    "class_specifier",
    "struct_specifier",
    "throw_statement",
    "switch_statement",
    "lambda_expression",
];

const RB_LITERAL_NODE_TYPES: &[&str] = &[
    "string",
    "chained_string",
    "bare_string",
    "integer",
    "float",
    "rational",
    "complex",
    "true",
    "false",
    "nil",
    "simple_symbol",
    "hash_key_symbol",
    "bare_symbol",
    "delimited_symbol",
    "character",
    "regex",
];

const RB_CONTROL_NODE_TYPES: &[&str] = &[
    "if",
    "unless",
    "while",
    "until",
    "for",
    "case",
    "case_match",
    "begin",
    "do_block",
    "block",
    "return",
    "yield",
    "rescue",
    "conditional",
    "method",
    "singleton_method",
    "class",
    "module",
];

const PASCAL_LITERAL_NODE_TYPES: &[&str] = &[
    "literalNumber",
    "literalString",
    "literalChar",
    "kTrue",
    "kFalse",
    "kNil",
];

const PASCAL_CONTROL_NODE_TYPES: &[&str] = &[
    "if",
    "ifElse",
    "while",
    "repeat",
    "for",
    "foreach",
    "case",
    "with",
    "try",
    "block",
    "goto",
    "defProc",
    "declProc",
    "declClass",
    "declType",
];

// Absolute cutoffs for the structural predicate.
const LITERAL_RATIO_CUTOFF: f64 = 0.80;
const NAMED_LEAF_COUNT_GATE: usize = 5;

/// Five AST-derived features. All five are recorded for audit/debug; only
/// `literal_leaf_ratio` and `named_leaf_count` gate the verdict.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypicalityFeatures {
    pub literal_leaf_ratio: f64,
    pub control_node_density: f64,
    pub ast_type_entropy: f64,
    pub unique_token_ratio: f64,
    pub named_leaf_count: usize,
}

/// The neutral sentinel returned on empty/unparseable input — a deliberately
/// "near-typical" default so downstream inference won't flag it as atypical.
const NEUTRAL: TypicalityFeatures = TypicalityFeatures {
    literal_leaf_ratio: 0.0,
    control_node_density: 0.0,
    ast_type_entropy: 0.0,
    unique_token_ratio: 0.0,
    named_leaf_count: 0,
};

/// `(literal_types, control_types)` node-type sets for `language`.
fn node_type_sets(language: Language) -> (HashSet<&'static str>, HashSet<&'static str>) {
    let (literal, control) = match language {
        Language::Python => (PY_LITERAL_NODE_TYPES, PY_CONTROL_NODE_TYPES),
        Language::Typescript => (TS_LITERAL_NODE_TYPES, TS_CONTROL_NODE_TYPES),
        // JavaScript shares TypeScript's literal/control node kinds (JS is a
        // syntactic subset).
        Language::Javascript => (TS_LITERAL_NODE_TYPES, TS_CONTROL_NODE_TYPES),
        Language::Go => (GO_LITERAL_NODE_TYPES, GO_CONTROL_NODE_TYPES),
        Language::Rust => (RUST_LITERAL_NODE_TYPES, RUST_CONTROL_NODE_TYPES),
        Language::C => (C_LITERAL_NODE_TYPES, C_CONTROL_NODE_TYPES),
        Language::Java => (JAVA_LITERAL_NODE_TYPES, JAVA_CONTROL_NODE_TYPES),
        Language::CSharp => (CS_LITERAL_NODE_TYPES, CS_CONTROL_NODE_TYPES),
        Language::Php => (PHP_LITERAL_NODE_TYPES, PHP_CONTROL_NODE_TYPES),
        Language::Cpp => (CPP_LITERAL_NODE_TYPES, CPP_CONTROL_NODE_TYPES),
        Language::Ruby => (RB_LITERAL_NODE_TYPES, RB_CONTROL_NODE_TYPES),
        Language::Pascal => (PASCAL_LITERAL_NODE_TYPES, PASCAL_CONTROL_NODE_TYPES),
    };
    (
        literal.iter().copied().collect(),
        control.iter().copied().collect(),
    )
}

/// `_is_leaf_equivalent_generic`: a named node treated as an atomic leaf for
/// ratio computation — either it has no children, or it is a literal type
/// (whose children are grammar-internal detail we don't descend into).
fn is_leaf_equivalent(node: Node, literal_types: &HashSet<&'static str>) -> bool {
    node.is_named() && (node.child_count() == 0 || literal_types.contains(node.kind()))
}

/// Shannon entropy (base-2) over the node-type counts: `-sum p*log2(p)`.
fn entropy(counts: &HashMap<String, usize>) -> f64 {
    let total: usize = counts.values().sum();
    if total == 0 {
        return 0.0;
    }
    let total = total as f64;
    let mut h = 0.0;
    for &c in counts.values() {
        let p = c as f64 / total;
        if p > 0.0 {
            h -= p * p.log2();
        }
    }
    h
}

/// Compute the five structural features for `source` in `language`.
///
/// Returns [`NEUTRAL`] on empty/whitespace-only input or when the tree yields
/// no named leaves (genuinely unparseable content). There is no early bail on
/// `ERROR`-root trees: tree-sitter still preserves typed literal children under
/// an `ERROR` subtree, and those leaves are countable.
pub fn compute_features(source: &str, language: Language) -> TypicalityFeatures {
    if source.trim().is_empty() {
        return NEUTRAL;
    }

    let (literal_types, control_types) = node_type_sets(language);

    let source_bytes = source.as_bytes();
    let tree = crate::scoring::ts_parse::parse(source, language)
        .expect("tree-sitter parse never returns None without a timeout");

    let mut named_leaf_count: usize = 0;
    let mut leaves_literal: usize = 0;
    let mut control_nodes: usize = 0;
    let mut node_type_counts: HashMap<String, usize> = HashMap::new();
    let mut token_counts: HashMap<String, usize> = HashMap::new();

    // `_walk_all_nodes(root, atomic_types=literal_types)`: DFS with an explicit
    // stack; push `reversed(children)`; do not descend into nodes whose kind is
    // in the literal set (they are atomic). Named AND anonymous nodes are walked.
    let mut stack: Vec<Node> = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        let kind = node.kind();

        if node.is_named() {
            *node_type_counts.entry(kind.to_string()).or_insert(0) += 1;
            if control_types.contains(kind) {
                control_nodes += 1;
            }
        }

        if is_leaf_equivalent(node, &literal_types) {
            named_leaf_count += 1;
            if literal_types.contains(kind) {
                leaves_literal += 1;
            }
            let range = node.byte_range();
            let token_text = if range.is_empty() {
                String::new()
            } else {
                String::from_utf8_lossy(&source_bytes[range]).into_owned()
            };
            *token_counts.entry(token_text).or_insert(0) += 1;
        }

        if !literal_types.contains(kind) {
            let mut cursor = node.walk();
            let children: Vec<Node> = node.children(&mut cursor).collect();
            for child in children.into_iter().rev() {
                stack.push(child);
            }
        }
    }

    if named_leaf_count == 0 {
        return NEUTRAL;
    }

    let total_lines = (source.matches('\n').count() + 1).max(1);
    let literal_leaf_ratio = leaves_literal as f64 / named_leaf_count as f64;
    let control_node_density = (control_nodes as f64 / total_lines as f64) * 100.0;
    let ast_type_entropy = entropy(&node_type_counts);
    let total_tokens: usize = token_counts.values().sum();
    let unique_token_ratio = if total_tokens == 0 {
        0.0
    } else {
        token_counts.len() as f64 / total_tokens as f64
    };

    TypicalityFeatures {
        literal_leaf_ratio,
        control_node_density,
        ast_type_entropy,
        unique_token_ratio,
        named_leaf_count,
    }
}

/// Stateless typicality predicate — absolute thresholds + file-level fallback.
/// Applied symmetrically at calibration (drop atypical hunks from the sampling
/// pool) and inference (short-circuit atypical hunks before BPE scoring).
pub struct TypicalityModel {
    language: Language,
}

impl TypicalityModel {
    pub fn new(language: Language) -> Self {
        Self { language }
    }

    /// Hunk-level check: `(true, features)` if structurally data-dominant.
    ///
    /// The v2 file-level fallback (`is_atypical_file`) was retired:
    /// its population — partial-array hunks below the hunk gate inside
    /// data-heavy files — is covered at row granularity by the scorer's
    /// data-row gate, which no longer silences *code* hunks in those files.
    pub fn is_atypical(&self, hunk: &str) -> (bool, TypicalityFeatures) {
        let features = compute_features(hunk, self.language);
        if features == NEUTRAL {
            return (false, features);
        }
        let is_atyp = features.named_leaf_count >= NAMED_LEAF_COUNT_GATE
            && features.literal_leaf_ratio > LITERAL_RATIO_CUTOFF;
        (is_atyp, features)
    }
}

#[cfg(test)]
mod tests;
