//! The five built-in shape primitives + shared tree-sitter helpers.
//!
//! Each primitive is a port of one Python module under
//! `engine/argot/scoring/scorers/`:
//! - [`namespace_jsd`] — receiver-namespace coverage divergence (JS distance),
//! - [`call_scope_fraction`] — fraction of calls at module scope,
//! - [`typical_call_density`] — under-coverage of cluster-typical callees,
//! - [`except_return_raise_ratio`] — return/raise ratio inside handler blocks,
//! - [`fall_through_guards`] — guard `if`s before the first `return`.

pub mod call_scope_fraction;
pub mod callee_distribution_under_coverage;
pub mod cluster_staple_deficit;
pub mod except_return_raise_ratio;
pub mod fall_through_guards;
pub mod namespace_jsd;
pub mod typical_call_density;

pub use call_scope_fraction::CallScopeFraction;
pub use callee_distribution_under_coverage::CalleeDistributionUnderCoverage;
pub use cluster_staple_deficit::ClusterStapleDeficit;
pub use except_return_raise_ratio::ExceptReturnRaiseRatio;
pub use fall_through_guards::FallThroughGuards;
pub use namespace_jsd::NamespaceJsd;
pub use typical_call_density::TypicalCallDensity;

use crate::scoring::adapters::Language;
use tree_sitter::{Node, Parser, Tree};

/// Parse `source` with the language grammar. Mirrors the Python primitives'
/// `parser.parse(source.encode("utf-8"))` with a broad except → `None`.
pub(crate) fn parse(source: &str, language: Language) -> Option<Tree> {
    let mut parser = Parser::new();
    let lang: tree_sitter::Language = match language {
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::Typescript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
    };
    parser.set_language(&lang).ok()?;
    parser.parse(source, None)
}

/// Pre-order DFS matching the Python `_walk` / `_walk_nodes` (stack, reversed
/// children), invoking `visit` on every node.
pub(crate) fn walk_preorder(root: Node, mut visit: impl FnMut(Node)) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        visit(node);
        let n = node.child_count();
        for i in (0..n).rev() {
            if let Some(c) = node.child(i) {
                stack.push(c);
            }
        }
    }
}

/// Call-expression node kinds per language (matches `_PY_CALL_TYPES` /
/// `_TS_CALL_TYPES` in `call_receiver.py`).
pub(crate) fn is_call_kind(kind: &str, language: Language) -> bool {
    match language {
        Language::Python => kind == "call",
        Language::Typescript => kind == "call_expression" || kind == "new_expression",
        Language::CSharp => kind == "invocation_expression" || kind == "object_creation_expression",
    }
}

/// Population mean and standard deviation (÷N). Matches `statistics.mean` +
/// `statistics.pstdev` (and the naive `sum/len` variants) for the inputs these
/// primitives produce; `pstdev` of a single value is 0, which this yields too.
pub(crate) fn population_mean_std(values: &[f64]) -> (f64, f64) {
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n;
    (mean, variance.sqrt())
}
