//! Fall-through-guard count — port of
//! `engine/argot/scoring/scorers/fall_through_guards.py`.
//!
//! Per function (`function_definition` / `function_declaration`), count
//! `if_statement` nodes whose `start_byte` is strictly less than the earliest
//! `return_statement`'s `start_byte` (0 if the function has no return).
//! Per-file mean of per-function counts; two-sided tail-z ramp.

use crate::scoring::adapters::Language;
use crate::scoring::shape_primitive::{Baseline, ShapePrimitive};
use crate::scoring::shape_primitives::{parse, population_mean_std};
use std::path::PathBuf;
use tree_sitter::Node;

const PY_FUNC: &str = "function_definition";
const TS_FUNC: &str = "function_declaration";
// Go: closest general case is the top-level `function_declaration` (methods
// aren't covered — this primitive is default-off in production).
const GO_FUNC: &str = "function_declaration";
const IF: &str = "if_statement";
const RETURN: &str = "return_statement";

const MIN_VALID_FILES: usize = 3;

/// Count guard `if`s before the first `return` in `func`'s subtree.
fn guards_before_return(func: Node) -> usize {
    let mut first_return: Option<usize> = None;
    let mut if_bytes: Vec<usize> = Vec::new();
    let mut stack = vec![func];
    while let Some(node) = stack.pop() {
        let kind = node.kind();
        if kind == RETURN {
            let b = node.start_byte();
            first_return = Some(match first_return {
                Some(fb) => fb.min(b),
                None => b,
            });
        } else if kind == IF {
            if_bytes.push(node.start_byte());
        }
        let n = node.child_count();
        for i in (0..n).rev() {
            if let Some(c) = node.child(i) {
                stack.push(c);
            }
        }
    }
    match first_return {
        None => 0,
        Some(fb) => if_bytes.iter().filter(|&&b| b < fb).count(),
    }
}

/// Mean guard count per function for `source`, or `None` if no functions.
fn file_avg_guards(source: &str, language: Language) -> Option<f64> {
    let tree = parse(source, language)?;
    let func_type = match language {
        Language::Python => PY_FUNC,
        Language::Typescript => TS_FUNC,
        Language::Go => GO_FUNC,
    };
    let mut counts: Vec<f64> = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.kind() == func_type {
            counts.push(guards_before_return(node) as f64);
        }
        let n = node.child_count();
        for i in (0..n).rev() {
            if let Some(c) = node.child(i) {
                stack.push(c);
            }
        }
    }
    if counts.is_empty() {
        return None;
    }
    Some(counts.iter().sum::<f64>() / counts.len() as f64)
}

/// Try Python grammar then TypeScript; first defined average wins.
fn score_hunk_avg(hunk: &str) -> Option<f64> {
    file_avg_guards(hunk, Language::Python).or_else(|| file_avg_guards(hunk, Language::Typescript))
}

/// Fall-through-guard count primitive.
#[derive(Default)]
pub struct FallThroughGuards;

impl ShapePrimitive for FallThroughGuards {
    fn name(&self) -> &str {
        "fall_through_guards"
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
        let mut avgs: Vec<f64> = Vec::new();
        for (_path, source) in cluster_files {
            if let Some(avg) = file_avg_guards(source, language) {
                avgs.push(avg);
            }
        }
        if avgs.len() < MIN_VALID_FILES {
            return None;
        }
        let (mean, std) = population_mean_std(&avgs);
        Some(Baseline::MeanStd { mean, std })
    }

    fn score(&self, hunk: &str, baseline: Option<&Baseline>, cluster_size: usize) -> f64 {
        let Some(Baseline::MeanStd { mean, std }) = baseline else {
            return 0.0;
        };
        if cluster_size < self.min_cluster_size() {
            return 0.0;
        }
        let Some(hunk_avg) = score_hunk_avg(hunk) else {
            return 0.0;
        };
        let tail_z = (hunk_avg - *mean) / std.max(1e-6);
        (tail_z.abs() - 1.0).max(0.0).min(self.cluster_bonus_clip())
    }
}
