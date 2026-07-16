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

const MIN_VALID_FILES: usize = 3;

/// Function-definition node kinds for `language` (Ruby methods come in plain
/// and singleton forms; Go/PHP have both free functions and methods).
fn func_kinds(language: Language) -> &'static [&'static str] {
    match language {
        Language::Python => &["function_definition"],
        Language::Typescript => &["function_declaration"],
        Language::Javascript => &["function_declaration"],
        Language::Go => &["function_declaration", "method_declaration"],
        Language::Rust => &["function_item"],
        Language::C => &["function_definition"],
        Language::Java => &["method_declaration"],
        Language::CSharp => &["method_declaration"],
        Language::Php => &["function_definition", "method_declaration"],
        Language::Cpp => &["function_definition"],
        Language::Ruby => &["method", "singleton_method"],
    }
}

/// Guard-`if` node kinds for `language`. Ruby's postfix `x if y` / `x unless y`
/// and their block forms all read as fall-through guards; Rust's `if` is an
/// expression, not a statement.
fn if_kinds(language: Language) -> &'static [&'static str] {
    match language {
        Language::Ruby => &["if", "if_modifier", "unless", "unless_modifier"],
        Language::Rust => &["if_expression"],
        _ => &["if_statement"],
    }
}

/// The `return` node kind for `language` (Ruby `return`, Rust
/// `return_expression`, everything else `return_statement`).
fn return_kind(language: Language) -> &'static str {
    match language {
        Language::Ruby => "return",
        Language::Rust => "return_expression",
        _ => "return_statement",
    }
}

/// Count guard `if`s before the first `return` in `func`'s subtree.
fn guards_before_return(func: Node, if_types: &[&str], return_type: &str) -> usize {
    let mut first_return: Option<usize> = None;
    let mut if_bytes: Vec<usize> = Vec::new();
    let mut stack = vec![func];
    while let Some(node) = stack.pop() {
        let kind = node.kind();
        if kind == return_type {
            let b = node.start_byte();
            first_return = Some(match first_return {
                Some(fb) => fb.min(b),
                None => b,
            });
        } else if if_types.contains(&kind) {
            if_bytes.push(node.start_byte());
        }
        for c in argot_lang::ts_parse::child_nodes(node).into_iter().rev() {
            stack.push(c);
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
    let funcs = func_kinds(language);
    let ifs = if_kinds(language);
    let ret = return_kind(language);
    let mut counts: Vec<f64> = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        if funcs.contains(&node.kind()) {
            counts.push(guards_before_return(node, ifs, ret) as f64);
        }
        for c in argot_lang::ts_parse::child_nodes(node).into_iter().rev() {
            stack.push(c);
        }
    }
    if counts.is_empty() {
        return None;
    }
    Some(counts.iter().sum::<f64>() / counts.len() as f64)
}

/// Probe each grammar in turn; first defined average wins. The hunk's language
/// is not known at score time, so every grammar is tried.
fn score_hunk_avg(hunk: &str) -> Option<f64> {
    file_avg_guards(hunk, Language::Python)
        .or_else(|| file_avg_guards(hunk, Language::Typescript))
        .or_else(|| file_avg_guards(hunk, Language::Go))
        .or_else(|| file_avg_guards(hunk, Language::Rust))
        .or_else(|| file_avg_guards(hunk, Language::C))
        .or_else(|| file_avg_guards(hunk, Language::Java))
        .or_else(|| file_avg_guards(hunk, Language::CSharp))
        .or_else(|| file_avg_guards(hunk, Language::Php))
        .or_else(|| file_avg_guards(hunk, Language::Cpp))
        .or_else(|| file_avg_guards(hunk, Language::Ruby))
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
