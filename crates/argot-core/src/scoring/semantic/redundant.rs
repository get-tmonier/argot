//! F1 · reinvention — "you already have this".
//!
//! For a function introduced by the diff, ask the [`SemanticIndex`] for its
//! nearest **cross-file** existing function and the **margin** by which that
//! neighbour stands out (`cos₁ − cos₂`). A high margin means one existing
//! function is distinctly the closest — a near-duplicate — rather than the query
//! merely resembling a cluster of peers. Absolute cosine can't do this job:
//! code embeddings are anisotropic (everything sits at cos 0.8–1.0), so the
//! *standout* margin is the signal, calibrated per-repo (see the fit-time bar in
//! `index::calibrate_margin_bar`).
//!
//! This module is pure scoring: it takes an embedding and returns a finding or
//! nothing. Extraction, embedding and `Hit` construction live in the check flow.
//! Findings are **advisory** — a real repo contains real duplication, which the
//! feature correctly surfaces; the evidence names the existing function so the
//! author judges.

use super::index::{FunctionRef, SemanticIndex};

/// Minimum nearest-cosine for a "you already have this" claim. Anisotropy keeps
/// almost all code above this, so the margin bar is the real gate — this only
/// rejects the degenerate case where even the closest match is genuinely far.
const ABS_SIMILARITY_FLOOR: f32 = 0.80;

/// A fired reinvention finding: the existing function this one duplicates.
#[derive(Debug, Clone)]
pub struct RedundantFinding {
    pub nearest_symbol: String,
    pub nearest_path: String,
    pub nearest_line: usize,
    /// Cosine to the nearest existing function (the "similarity").
    pub similarity: f32,
    /// How far the nearest neighbour stands out (`cos₁ − cos₂`).
    pub margin: f32,
}

/// Scores diff-defined functions against a repo's existing functions.
pub struct RedundantScorer<'a> {
    index: &'a SemanticIndex,
    /// Per-repo calibrated margin bar; a function fires only if its margin
    /// exceeds this.
    margin_bar: f32,
}

impl<'a> RedundantScorer<'a> {
    pub fn new(index: &'a SemanticIndex, margin_bar: f32) -> Self {
        Self { index, margin_bar }
    }

    /// Evaluate one diff-defined function. `query` is its L2-normalised
    /// embedding; `func` carries its identity for gating and same-file exclusion.
    /// Returns `Some` when the function reinvents an existing one.
    pub fn evaluate(&self, func: &FunctionRef, query: &[f32]) -> Option<RedundantFinding> {
        if self.index.is_empty() || !is_reinvention_candidate(&func.symbol, &func.path) {
            return None;
        }
        // Nearest two *cross-file* neighbours (same-file matches are overloads /
        // adjacent helpers, a known false-alarm driver).
        let neigh = self.index.nearest(query, 2, |e| e.path != func.path);
        let best = *neigh.first()?;
        let best_entry = self.index.entry(best.entry_index);
        // A near-duplicate that keeps the *same name* in another file is almost
        // always a move/rename, not a reinvention — don't flag refactors.
        if eq_ignore_ascii_case(&best_entry.symbol, &func.symbol) {
            return None;
        }
        if best.cosine < ABS_SIMILARITY_FLOOR {
            return None;
        }
        let second = neigh.get(1).map(|n| n.cosine).unwrap_or(0.0);
        let margin = best.cosine - second;
        if margin < self.margin_bar {
            return None;
        }
        Some(RedundantFinding {
            nearest_symbol: best_entry.symbol.clone(),
            nearest_path: best_entry.path.clone(),
            nearest_line: best_entry.line,
            similarity: best.cosine,
            margin,
        })
    }
}

fn eq_ignore_ascii_case(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.eq_ignore_ascii_case(b)
}

/// Whether a diff-defined function is a candidate for reinvention flagging.
/// Excludes boilerplate whose semantic similarity is legitimate and expected:
/// dunder/magic methods (`__init__`, `__eq__`, …) and functions defined in test
/// / fixture files (test doubles routinely reimplement helpers on purpose).
fn is_reinvention_candidate(symbol: &str, path: &str) -> bool {
    !is_dunder(symbol) && !is_test_path(path)
}

/// A `__dunder__` / magic name — language-structural boilerplate, not authored
/// logic. Cheap universal check (harmless on languages without dunders).
fn is_dunder(symbol: &str) -> bool {
    symbol.len() >= 5 && symbol.starts_with("__") && symbol.ends_with("__")
}

/// Path-component heuristic for test / fixture files. Component-level (not raw
/// substring) so `src/testing_utils.py` is *not* treated as a test file. Shared
/// with the placement scorer (test doubles are legitimately relocated too).
pub(super) fn is_test_path(path: &str) -> bool {
    for comp in path.split('/') {
        let c = comp.to_ascii_lowercase();
        if c == "test" || c == "tests" || c == "spec" || c == "specs" || c == "__tests__" {
            return true;
        }
        let stem = c.split('.').next().unwrap_or(&c);
        if stem == "conftest"
            || stem.starts_with("test_")
            || stem.ends_with("_test")
            || c.contains(".test.")
            || c.contains(".spec.")
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::semantic::index::IndexEntry;

    fn unit(v: Vec<f32>) -> Vec<f32> {
        let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.into_iter().map(|x| x / n).collect()
    }

    fn entry(symbol: &str, path: &str, vec: Vec<f32>) -> IndexEntry {
        IndexEntry {
            symbol: symbol.into(),
            path: path.into(),
            line: 1,
            vec: unit(vec),
        }
    }

    fn func(symbol: &str, path: &str) -> FunctionRef {
        FunctionRef {
            symbol: symbol.into(),
            path: path.into(),
            line: 10,
            end_line: 15,
            text: String::new(),
        }
    }

    /// An index where entry `slugify` sits alone in one direction and a cluster
    /// of unrelated code sits elsewhere.
    fn index() -> SemanticIndex {
        SemanticIndex {
            dim: 3,
            entries: vec![
                entry("slugify", "src/utils/text.py", vec![1.0, 0.0, 0.0]),
                entry("parse_config", "src/cfg.py", vec![0.0, 1.0, 0.0]),
                entry("load_yaml", "src/cfg.py", vec![0.0, 0.9, 0.1]),
            ],
        }
    }

    #[test]
    fn fires_on_near_duplicate_across_files() {
        let idx = index();
        let scorer = RedundantScorer::new(&idx, 0.3);
        // A new function very close to `slugify`, in a different file.
        let q = unit(vec![0.98, 0.02, 0.0]);
        let finding = scorer.evaluate(&func("normalize_slug", "src/api/handlers.py"), &q);
        let f = finding.expect("near-duplicate fires");
        assert_eq!(f.nearest_symbol, "slugify");
        assert!(f.similarity > 0.9 && f.margin > 0.3);
    }

    #[test]
    fn does_not_fire_on_distinct_function() {
        let idx = index();
        let scorer = RedundantScorer::new(&idx, 0.3);
        // Sits between the two cfg functions — no single standout (low margin).
        let q = unit(vec![0.0, 0.95, 0.31]);
        assert!(scorer
            .evaluate(&func("brand_new_thing", "src/api/handlers.py"), &q)
            .is_none());
    }

    #[test]
    fn same_file_match_is_excluded() {
        let idx = index();
        let scorer = RedundantScorer::new(&idx, 0.3);
        let q = unit(vec![0.98, 0.02, 0.0]);
        // Candidate lives in slugify's own file → its only near-dup is same-file.
        assert!(scorer
            .evaluate(&func("slug2", "src/utils/text.py"), &q)
            .is_none());
    }

    #[test]
    fn same_name_is_treated_as_move_not_reinvention() {
        let idx = index();
        let scorer = RedundantScorer::new(&idx, 0.3);
        let q = unit(vec![0.98, 0.02, 0.0]);
        // Same symbol name, different file → a move/rename, not a reinvention.
        assert!(scorer
            .evaluate(&func("slugify", "src/api/handlers.py"), &q)
            .is_none());
    }

    #[test]
    fn dunder_and_test_paths_are_gated() {
        assert!(!is_reinvention_candidate("__init__", "src/a.py"));
        assert!(!is_reinvention_candidate("helper", "tests/test_a.py"));
        assert!(!is_reinvention_candidate("helper", "src/a.test.ts"));
        assert!(!is_reinvention_candidate("helper", "spec/thing.py"));
        // Not a test file despite containing "test" in a component.
        assert!(is_reinvention_candidate("helper", "src/testing_utils.py"));
        assert!(is_reinvention_candidate("normalize", "src/utils/text.py"));
    }
}
