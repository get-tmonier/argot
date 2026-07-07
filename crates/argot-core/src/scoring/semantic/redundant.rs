//! F1 · reinvention — "you already have this".
//!
//! For a function introduced by the diff, ask the [`SemanticIndex`] for its
//! nearest **cross-file** existing function and fire on either of two signals:
//!
//! - **callee-confirm** (the workhorse): the nearest neighbour is very close
//!   (`cos₁ ≥ 0.85`) AND they share a meaningful fraction of **callees**. Code
//!   embeddings are anisotropic (everything sits at cos 0.8–1.0), so absolute
//!   cosine alone over-fires; the shared-callee overlap confirms it's a real
//!   reinvention (genuinely-new code shares ~0 callees with its nearest match, a
//!   reinvention ~half). This lifts recall from ~20% to ~60–70% at ~1% over-fire.
//! - **margin** (secondary): the neighbour stands out distinctly (`cos₁ − cos₂`
//!   above a per-repo bar) — catches callee-less standouts the first path misses.
//!
//! This module is pure scoring: it takes an embedding and returns a finding or
//! nothing. Extraction, embedding and `Hit` construction live in the check flow.
//! Findings are **advisory** — a real repo contains real duplication, which the
//! feature correctly surfaces; the evidence names the existing function so the
//! author judges.

use super::index::{FunctionRef, SemanticIndex};

/// Minimum nearest-cosine for the *margin* path — a "you already have this"
/// claim needs a genuinely close match. Anisotropy keeps most code above this,
/// so on the margin path the bar is the real gate.
const ABS_SIMILARITY_FLOOR: f32 = 0.80;

/// The **callee-confirm** path (the frontier-breaker): embedding retrieves the
/// candidate, shared callees confirm it's a real reinvention rather than an
/// anisotropy near-match. A diff function fires if its nearest neighbour is very
/// close AND they call a meaningful fraction of the same functions — genuinely-
/// new code shares ~0 callees with its nearest match, a reinvention ~half.
/// Absolute bars (stable across corpora): validated on rich + scrapy at ~71–80%
/// recall / ~1–2% over-fire vs the margin path's ~20% (see `P5-tuning.md`).
const CONFIRM_SIMILARITY: f32 = 0.85;
const CALLEE_OVERLAP_BAR: f32 = 0.15;
/// A function with fewer callees than this can't form a reliable callee-Jaccard
/// (a 1-callee fn matching a 1-callee fn is 100% overlap by luck), so the
/// callee-confirm path needs both sides to have at least this many callees.
const MIN_CALLEES_FOR_CONFIRM: usize = 2;

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
        let second = neigh.get(1).map(|n| n.cosine).unwrap_or(0.0);
        let margin = best.cosine - second;
        // Margin path: the nearest neighbour stands out distinctly. Callee-confirm
        // path: it's very close AND shares a meaningful fraction of callees. Either
        // fires; the callee path recovers the reinventions the margin misses when
        // the repo already has near-duplicate clusters (diluted margin).
        let margin_fires = best.cosine >= ABS_SIMILARITY_FLOOR && margin >= self.margin_bar;
        let callee_jac = callee_jaccard(&func.callees, &best_entry.callees);
        let callee_fires = best.cosine >= CONFIRM_SIMILARITY
            && func.callees.len() >= MIN_CALLEES_FOR_CONFIRM
            && best_entry.callees.len() >= MIN_CALLEES_FOR_CONFIRM
            && callee_jac >= CALLEE_OVERLAP_BAR;
        if !margin_fires && !callee_fires {
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

/// Jaccard overlap of two callee sets (both are sorted, deduped). 0 when both
/// are empty.
fn callee_jaccard(a: &[String], b: &[String]) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let sa: std::collections::BTreeSet<&str> = a.iter().map(String::as_str).collect();
    let sb: std::collections::BTreeSet<&str> = b.iter().map(String::as_str).collect();
    let inter = sa.intersection(&sb).count();
    let union = sa.union(&sb).count();
    if union == 0 {
        0.0
    } else {
        inter as f32 / union as f32
    }
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
        entry_c(symbol, path, vec, &[])
    }

    fn entry_c(symbol: &str, path: &str, vec: Vec<f32>, callees: &[&str]) -> IndexEntry {
        IndexEntry {
            symbol: symbol.into(),
            path: path.into(),
            line: 1,
            vec: unit(vec),
            callees: callees.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn func(symbol: &str, path: &str) -> FunctionRef {
        func_c(symbol, path, &[])
    }

    fn func_c(symbol: &str, path: &str, callees: &[&str]) -> FunctionRef {
        FunctionRef {
            symbol: symbol.into(),
            path: path.into(),
            line: 10,
            end_line: 15,
            text: String::new(),
            callees: callees.iter().map(|s| s.to_string()).collect(),
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

    /// Two near-duplicate existing functions dilute the margin below the bar, so
    /// the margin path stays quiet — but the candidate shares the original's
    /// callees, so the callee-confirm path fires. This is the frontier-breaker.
    fn diluted_index() -> SemanticIndex {
        SemanticIndex {
            dim: 3,
            entries: vec![
                entry_c(
                    "format_price",
                    "src/money.py",
                    vec![1.0, 0.0, 0.0],
                    &["round", "currency", "symbol"],
                ),
                entry_c(
                    "format_amount",
                    "src/money.py",
                    vec![0.99, 0.02, 0.0],
                    &["round", "currency"],
                ),
            ],
        }
    }

    #[test]
    fn callee_confirmation_fires_when_margin_is_diluted() {
        let idx = diluted_index();
        let scorer = RedundantScorer::new(&idx, 0.5); // bar high → margin path can't fire
        let q = unit(vec![0.995, 0.01, 0.0]); // cos1≈1.0, cos2≈0.99 → tiny margin
        let f = scorer
            .evaluate(
                &func_c(
                    "render_price",
                    "src/ui/views.py",
                    &["round", "currency", "symbol"],
                ),
                &q,
            )
            .expect("callee-confirm fires despite a diluted margin");
        assert_eq!(f.nearest_symbol, "format_price");
        assert!(
            f.margin < 0.5,
            "the margin path would NOT have fired: {}",
            f.margin
        );
    }

    #[test]
    fn callee_confirm_needs_shared_callees() {
        // Same close embedding, but no shared callees → not a reinvention, and the
        // margin is diluted, so nothing fires.
        let idx = diluted_index();
        let scorer = RedundantScorer::new(&idx, 0.5);
        let q = unit(vec![0.995, 0.01, 0.0]);
        assert!(scorer
            .evaluate(
                &func_c(
                    "compute_layout",
                    "src/ui/views.py",
                    &["measure", "wrap", "clamp"]
                ),
                &q,
            )
            .is_none());
    }
}
