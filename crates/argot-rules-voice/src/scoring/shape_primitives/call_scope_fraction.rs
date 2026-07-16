//! Call-scope distribution.
//!
//! Fraction of call nodes with no enclosing function ancestor (module scope)
//! vs all call nodes. Two-sided tail-z ramp.
//!
//! The scope boundary is grammar-specific: tree-sitter-python uses
//! `function_definition`, tree-sitter-typescript uses `function_declaration`.
//! (Using tree-sitter-python's `function_definition` for both grammars would
//! make the fraction constantly 1.0 on TypeScript and the primitive always
//! abstain there — hence the per-grammar boundary.)

use crate::scoring::adapters::Language;
use crate::scoring::shape_primitive::{Baseline, ShapePrimitive};
use crate::scoring::shape_primitives::{is_call_kind, parse, population_mean_std, walk_preorder};
use std::cell::Cell;
use std::path::PathBuf;
use tree_sitter::Node;

/// Boundary kinds between module scope and nested scope, per grammar.
const FUNCTION_BOUNDARIES: &[&str] = &["function_definition", "function_declaration"];

/// Minimum files with ≥1 call for the baseline to be trusted.
const MIN_FILES: usize = 3;

fn has_function_ancestor(node: Node) -> bool {
    let mut parent = node.parent();
    while let Some(p) = parent {
        if FUNCTION_BOUNDARIES.contains(&p.kind()) {
            return true;
        }
        parent = p.parent();
    }
    false
}

/// Module-scope call fraction, or `None` if the source has 0 call nodes.
fn fraction_module_scope(source: &str, language: Language) -> Option<f64> {
    let tree = parse(source, language)?;
    let mut total = 0usize;
    let mut module_scope = 0usize;
    walk_preorder(tree.root_node(), |node| {
        if is_call_kind(node.kind(), language) {
            total += 1;
            if !has_function_ancestor(node) {
                module_scope += 1;
            }
        }
    });
    if total == 0 {
        return None;
    }
    Some(module_scope as f64 / total as f64)
}

/// Fraction-of-calls-at-module-scope primitive. Language is captured on fit
/// (interior mutability) and reused at score time.
#[derive(Default)]
pub struct CallScopeFraction {
    language: Cell<Option<Language>>,
}

impl ShapePrimitive for CallScopeFraction {
    fn name(&self) -> &str {
        "call_scope_fraction"
    }
    fn min_cluster_size(&self) -> usize {
        10
    }
    fn cluster_bonus_clip(&self) -> f64 {
        5.0
    }

    fn set_language(&self, language: Language) {
        self.language.set(Some(language));
    }

    fn fit_cluster_baseline(
        &self,
        cluster_files: &[(PathBuf, String)],
        language: Language,
    ) -> Option<Baseline> {
        self.language.set(Some(language));
        let mut fractions: Vec<f64> = Vec::new();
        for (_path, source) in cluster_files {
            if let Some(frac) = fraction_module_scope(source, language) {
                fractions.push(frac);
            }
        }
        if fractions.len() < MIN_FILES {
            return None;
        }
        let (mean, std) = population_mean_std(&fractions);
        Some(Baseline::MeanStd { mean, std })
    }

    fn score(&self, hunk: &str, baseline: Option<&Baseline>, cluster_size: usize) -> f64 {
        let Some(Baseline::MeanStd { mean, std }) = baseline else {
            return 0.0;
        };
        if cluster_size < self.min_cluster_size() {
            return 0.0;
        }
        let Some(language) = self.language.get() else {
            return 0.0;
        };
        let Some(hunk_frac) = fraction_module_scope(hunk, language) else {
            return 0.0;
        };
        let tail_z = (hunk_frac - *mean) / std.max(1e-6);
        (tail_z.abs() - 1.0).max(0.0).min(self.cluster_bonus_clip())
    }
}

#[cfg(test)]
mod tests;
