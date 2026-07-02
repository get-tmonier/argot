//! Except-block return/raise ratio — port of
//! `engine/argot/scoring/scorers/except_return_raise_ratio.py`.
//!
//! Within each handler subtree (`except_clause` / `catch_clause`),
//! ratio = returns / (returns + raises). Nested handlers are counted
//! independently (each handler's full subtree is walked). Per-file mean of
//! handler ratios; two-sided tail-z ramp.

use crate::scoring::adapters::Language;
use crate::scoring::shape_primitive::{Baseline, ShapePrimitive};
use crate::scoring::shape_primitives::{parse, population_mean_std};
use std::path::PathBuf;
use tree_sitter::Node;

const PY_HANDLER: &str = "except_clause";
const TS_HANDLER: &str = "catch_clause";
const JAVA_HANDLER: &str = "catch_clause";
const RETURN: &str = "return_statement";
const PY_RAISE: &str = "raise_statement";
const TS_RAISE: &str = "throw_statement";
const JAVA_RAISE: &str = "throw_statement";
const CS_HANDLER: &str = "catch_clause";
const RETURN: &str = "return_statement";
const PY_RAISE: &str = "raise_statement";
const TS_RAISE: &str = "throw_statement";
const CS_RAISE: &str = "throw_statement";
const PHP_HANDLER: &str = "catch_clause";
const RETURN: &str = "return_statement";
const PY_RAISE: &str = "raise_statement";
const TS_RAISE: &str = "throw_statement";
const PHP_RAISE: &str = "throw_expression";
const CPP_HANDLER: &str = "catch_clause";
const RETURN: &str = "return_statement";
const PY_RAISE: &str = "raise_statement";
const TS_RAISE: &str = "throw_statement";
const CPP_RAISE: &str = "throw_statement";

const MIN_VALID_FILES: usize = 3;

/// Count `(returns, raises)` in the whole subtree rooted at `root`.
fn count_returns_raises(root: Node, raise_type: &str) -> (usize, usize) {
    let mut returns = 0usize;
    let mut raises = 0usize;
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let kind = node.kind();
        if kind == RETURN {
            returns += 1;
        } else if kind == raise_type {
            raises += 1;
        }
        let n = node.child_count();
        for i in (0..n).rev() {
            if let Some(c) = node.child(i) {
                stack.push(c);
            }
        }
    }
    (returns, raises)
}

/// Sum `(returns, raises)` across every handler subtree under `root`. Nested
/// handlers are walked independently (double-counting is intentional, matching
/// the Python `_count_in_handlers`).
fn count_in_handlers(root: Node, handler_type: &str, raise_type: &str) -> (usize, usize) {
    let mut returns = 0usize;
    let mut raises = 0usize;
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == handler_type {
            let (r, x) = count_returns_raises(node, raise_type);
            returns += r;
            raises += x;
        }
        let n = node.child_count();
        for i in (0..n).rev() {
            if let Some(c) = node.child(i) {
                stack.push(c);
            }
        }
    }
    (returns, raises)
}

fn ratio_for_source(source: &str, language: Language) -> Option<f64> {
    let tree = parse(source, language)?;
    let (handler, raise) = match language {
        Language::Python => (PY_HANDLER, PY_RAISE),
        Language::Typescript => (TS_HANDLER, TS_RAISE),
        // Go has no exception-handler construct (errors are values); neither
        // kind exists in the Go grammar, so this primitive stays inert for Go.
        Language::Go => (TS_HANDLER, TS_RAISE),
        // Rust likewise has no exception handlers (errors are values, `?` /
        // `Result`); neither kind exists in the Rust grammar, so this primitive
        // stays inert for Rust.
        Language::Rust => (TS_HANDLER, TS_RAISE),
        // C has no exception-handling construct, so this ratio is undefined:
        // no handler blocks means no signal.
        Language::C => return None,
        Language::Java => (JAVA_HANDLER, JAVA_RAISE),
        Language::CSharp => (CS_HANDLER, CS_RAISE),
        Language::Php => (PHP_HANDLER, PHP_RAISE),
        Language::Cpp => (CPP_HANDLER, CPP_RAISE),
    };
    let (returns, raises) = count_in_handlers(tree.root_node(), handler, raise);
    let total = returns + raises;
    if total == 0 {
        return None;
    }
    Some(returns as f64 / total as f64)
}

/// Try Python grammar, then TypeScript, then Java; first defined ratio wins.
fn ratio_for_hunk(hunk: &str) -> Option<f64> {
    ratio_for_source(hunk, Language::Python)
        .or_else(|| ratio_for_source(hunk, Language::Typescript))
        .or_else(|| ratio_for_source(hunk, Language::Java))
/// Try Python grammar, then TypeScript, then C#; first defined ratio wins.
fn ratio_for_hunk(hunk: &str) -> Option<f64> {
    ratio_for_source(hunk, Language::Python)
        .or_else(|| ratio_for_source(hunk, Language::Typescript))
        .or_else(|| ratio_for_source(hunk, Language::CSharp))
/// Try Python grammar, then TypeScript, then PHP; first defined ratio wins.
fn ratio_for_hunk(hunk: &str) -> Option<f64> {
    ratio_for_source(hunk, Language::Python)
        .or_else(|| ratio_for_source(hunk, Language::Typescript))
        .or_else(|| ratio_for_source(hunk, Language::Php))
/// Try Python grammar then TypeScript then C++; first defined ratio wins.
fn ratio_for_hunk(hunk: &str) -> Option<f64> {
    ratio_for_source(hunk, Language::Python)
        .or_else(|| ratio_for_source(hunk, Language::Typescript))
        .or_else(|| ratio_for_source(hunk, Language::Cpp))
}

/// Except-block return/raise ratio primitive.
#[derive(Default)]
pub struct ExceptReturnRaiseRatio;

impl ShapePrimitive for ExceptReturnRaiseRatio {
    fn name(&self) -> &str {
        "except_return_raise_ratio"
    }
    fn min_cluster_size(&self) -> usize {
        10
    }
    fn cluster_bonus_clip(&self) -> f64 {
        5.0
    }

    fn fit_cluster_baseline(
        &self,
        cluster_files: &[(PathBuf, String)],
        language: Language,
    ) -> Option<Baseline> {
        let mut ratios: Vec<f64> = Vec::new();
        for (_path, source) in cluster_files {
            if let Some(r) = ratio_for_source(source, language) {
                ratios.push(r);
            }
        }
        if ratios.len() < MIN_VALID_FILES {
            return None;
        }
        let (mean, std) = population_mean_std(&ratios);
        Some(Baseline::MeanStd { mean, std })
    }

    fn score(&self, hunk: &str, baseline: Option<&Baseline>, cluster_size: usize) -> f64 {
        let Some(Baseline::MeanStd { mean, std }) = baseline else {
            return 0.0;
        };
        if cluster_size < self.min_cluster_size() {
            return 0.0;
        }
        let Some(hunk_ratio) = ratio_for_hunk(hunk) else {
            return 0.0;
        };
        let tail_z = (hunk_ratio - *mean) / std.max(1e-6);
        (tail_z.abs() - 1.0).max(0.0).min(self.cluster_bonus_clip())
    }
}
