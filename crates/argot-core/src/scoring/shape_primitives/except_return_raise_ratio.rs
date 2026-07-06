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

const MIN_VALID_FILES: usize = 3;

/// The exception-handler clause node kind for `language`, or `None` for
/// languages with no exception-handling construct (Go, Rust, C — errors are
/// values there, so there are no handler blocks to measure).
fn handler_kind(language: Language) -> Option<&'static str> {
    match language {
        Language::Python => Some("except_clause"),
        Language::Typescript
        | Language::Javascript
        | Language::Java
        | Language::CSharp
        | Language::Cpp
        | Language::Php => Some("catch_clause"),
        Language::Ruby => Some("rescue"),
        Language::Go | Language::Rust | Language::C => None,
    }
}

/// The `return`-statement node kind for `language`.
fn return_kind(language: Language) -> &'static str {
    match language {
        Language::Ruby => "return",
        _ => "return_statement",
    }
}

/// Whether `node` raises. Python/TS have a dedicated statement node; Ruby
/// raises via a `raise`/`fail` method call, so it needs the source text.
fn is_raise(node: Node, src: &[u8], language: Language) -> bool {
    match language {
        Language::Python => node.kind() == "raise_statement",
        // PHP raises via a throw *expression*, not a statement.
        Language::Php => node.kind() == "throw_expression",
        Language::Ruby => {
            node.kind() == "call"
                && node.child_by_field_name("receiver").is_none()
                && node
                    .child_by_field_name("method")
                    .map(|m| {
                        let text = &src[m.byte_range()];
                        text == b"raise" || text == b"fail"
                    })
                    .unwrap_or(false)
        }
        // TypeScript, Java, C#, C++ all raise via a throw statement. Go, Rust
        // and C never reach here (they have no handler kind).
        _ => node.kind() == "throw_statement",
    }
}

/// Count `(returns, raises)` in the whole subtree rooted at `root`.
fn count_returns_raises(root: Node, src: &[u8], language: Language) -> (usize, usize) {
    let return_type = return_kind(language);
    let mut returns = 0usize;
    let mut raises = 0usize;
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == return_type {
            returns += 1;
        } else if is_raise(node, src, language) {
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
fn count_in_handlers(root: Node, src: &[u8], language: Language) -> (usize, usize) {
    let handler_type = match handler_kind(language) {
        Some(h) => h,
        None => return (0, 0),
    };
    let mut returns = 0usize;
    let mut raises = 0usize;
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == handler_type {
            let (r, x) = count_returns_raises(node, src, language);
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
    // Languages with no exception-handling construct (Go, Rust, C) have no
    // handler blocks, so the ratio is undefined — no signal.
    handler_kind(language)?;
    let (returns, raises) = count_in_handlers(tree.root_node(), source.as_bytes(), language);
    let total = returns + raises;
    if total == 0 {
        return None;
    }
    Some(returns as f64 / total as f64)
}

/// Probe the exception-having grammars in turn; first defined ratio wins.
/// The hunk's language is not known at score time, so we try each grammar
/// that has an exception-handler construct.
fn ratio_for_hunk(hunk: &str) -> Option<f64> {
    ratio_for_source(hunk, Language::Python)
        .or_else(|| ratio_for_source(hunk, Language::Typescript))
        .or_else(|| ratio_for_source(hunk, Language::Java))
        .or_else(|| ratio_for_source(hunk, Language::CSharp))
        .or_else(|| ratio_for_source(hunk, Language::Php))
        .or_else(|| ratio_for_source(hunk, Language::Cpp))
        .or_else(|| ratio_for_source(hunk, Language::Ruby))
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
