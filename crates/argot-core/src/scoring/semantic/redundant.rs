//! F1 · reinvention — "you already have this".
//!
//! For a function introduced by the diff, ask the [`SemanticIndex`] for its
//! nearest **cross-file** existing function. The embedding *retrieves* the
//! candidate; cheap **structural** signals *confirm* it. Code embeddings are
//! anisotropic (everything sits at cos 0.7–1.0), so cosine alone over-fires — a
//! near-match only fires when it also agrees structurally on one of:
//!
//! - **callee overlap** — the two functions call a meaningful fraction of the
//!   same functions. A reinvention reuses the same helpers; genuinely-new code
//!   shares ~0 callees with its nearest match.
//! - **weighted-subtoken overlap** — their identifiers, split into subtokens
//!   (`getUserName` → `user`, `name`) and weighted by corpus rarity (IDF), agree
//!   on the *rare, domain-specific* vocabulary. A shared rare token
//!   (`east_asian_width`) is strong evidence; shared ubiquitous ones (`self`,
//!   `get`, `return`) carry ~0 weight, so no per-language stop-list is needed.
//!
//! Two tiers. The **normal** tier fires on a close match (`cos₁ ≥ 0.78`) with
//! *moderate* structural agreement. The **strong** tier *rescues* a slightly more
//! distant match (`cos₁ ≥ 0.70`) when the structural agreement is *high* — a
//! heavily-reworded reinvention embeds further from the original but still reuses
//! its rare vocabulary / helpers. Tuned on rich + scrapy to ~86% recall at ~2%
//! over-fire (see `.scratch/semantic-layer/P5-tuning.md`).
//!
//! This module is pure scoring: it takes an embedding and returns a finding or
//! nothing. Extraction, embedding and `Hit` construction live in the check flow.
//! Findings are **advisory** — a real repo contains real duplication, which the
//! feature correctly surfaces; the evidence names the existing function so the
//! author judges.

use std::collections::{BTreeSet, HashMap};

use super::index::{FunctionRef, IndexEntry, SemanticIndex};

/// Normal tier — a close embedding match with *moderate* structural agreement.
const NORMAL_SIMILARITY: f32 = 0.78;
const NORMAL_SUBTOKEN_BAR: f32 = 0.40;
const NORMAL_CALLEE_BAR: f32 = 0.12;
/// The callee path needs both sides to have at least this many callees — a
/// 1-callee fn matching a 1-callee fn is 100% overlap by luck, not evidence.
const NORMAL_MIN_CALLEES: usize = 2;

/// Strong (rescue) tier — a slightly more distant match that *strongly* agrees
/// structurally. Catches heavily-reworded reinventions that embed further from
/// the original but still reuse its rare vocabulary / helpers. Firing this low on
/// cosine is only safe *because* the structural bars are high.
const STRONG_SIMILARITY: f32 = 0.70;
const STRONG_SUBTOKEN_BAR: f32 = 0.52;
const STRONG_CALLEE_BAR: f32 = 0.30;
const STRONG_MIN_CALLEES: usize = 3;

/// Rare-callee path: a small function (1–2 callees) can't clear the callee
/// `MIN_CALLEES` guard, and its subtokens are often generic (a geometry helper's
/// `point`/`vector` vocabulary). But if it shares a **rare** callee with its match
/// — a specific helper called by only a sliver of the repo — that single shared
/// call is strong reinvention evidence (a faithful reimplementation calls the same
/// specific helper; overlap on a rare callee is not luck the way overlap on one
/// ubiquitous callee is). Fires at the strong-tier cosine with no min-callee count.
/// "Rare" = called by ≤ this fraction of corpus functions (floored for tiny repos).
const RARE_CALLEE_DF_FRACTION: f64 = 0.012;
const RARE_CALLEE_DF_FLOOR: u32 = 4;

/// The lowest cosine at which a reinvention can fire (the strong-tier floor).
/// Exposed so the check flow can report it as the finding's informational
/// "threshold".
pub(crate) const MIN_SIMILARITY_TO_FIRE: f32 = STRONG_SIMILARITY;

/// Composition-gate escape hatch. The composition gate suppresses a match when
/// the candidate calls (or is called by) the matched function — a well-factored
/// family (`pointOnPolygon` calling `pointOnLineSegment`) reuses vocabulary while
/// *using*, not duplicating, the helper. But that family case is a genuinely
/// different function: it agrees on the match only *moderately*. When the
/// candidate instead agrees *strongly* — a high cosine AND high rare-vocabulary
/// (subtoken) overlap — it is a near-copy, and the shared call is a name
/// coincidence (typically the reinvention wraps an *imported* helper that happens
/// to share the matched function's name: a `fresh()` wrapper calling the `fresh`
/// npm module, not the repo's `fresh`). Suppressing that would miss a true
/// duplicate, so the gate steps aside above both bars.
const COMPOSITION_NEAR_DUP_SIM: f32 = 0.82;

/// A fired reinvention finding: the existing function this one duplicates.
#[derive(Debug, Clone)]
pub struct RedundantFinding {
    pub nearest_symbol: String,
    pub nearest_path: String,
    pub nearest_line: usize,
    /// Cosine to the nearest existing function (the "similarity").
    pub similarity: f32,
}

/// Scores diff-defined functions against a repo's existing functions. Holds the
/// corpus subtoken IDF (built once from the index), so subtoken overlap weights
/// rare, domain-specific vocabulary and discounts ubiquitous tokens (`self`,
/// `get`, `return`) without any hand-tuned stop-list.
pub struct RedundantScorer<'a> {
    index: &'a SemanticIndex,
    subtoken_idf: HashMap<String, f32>,
    /// IDF for a subtoken not seen in the corpus (df = 0) — the max weight.
    default_idf: f32,
    /// Per-callee corpus document frequency (how many functions call it) and the
    /// rarity cutoff below which a shared callee alone confirms a reinvention.
    callee_df: HashMap<String, u32>,
    rare_callee_df: u32,
}

impl<'a> RedundantScorer<'a> {
    pub fn new(index: &'a SemanticIndex) -> Self {
        let n_docs = index.entries.len().max(1) as f64;
        let subtoken_idf = corpus_idf(index.entries.iter().map(|e| &e.subtokens), n_docs);
        let mut callee_df: HashMap<String, u32> = HashMap::new();
        for e in &index.entries {
            for c in &e.callees {
                *callee_df.entry(c.clone()).or_insert(0) += 1;
            }
        }
        let rare_callee_df =
            ((RARE_CALLEE_DF_FRACTION * n_docs).ceil() as u32).max(RARE_CALLEE_DF_FLOOR);
        Self {
            index,
            subtoken_idf,
            default_idf: idf_of(0, n_docs),
            callee_df,
            rare_callee_df,
        }
    }

    /// Evaluate one diff-defined function. `query` is its L2-normalised
    /// embedding; `func` carries its identity for gating and same-file exclusion.
    /// Returns `Some` when the function reinvents an existing one.
    pub fn evaluate(&self, func: &FunctionRef, query: &[f32]) -> Option<RedundantFinding> {
        if self.index.is_empty() || !is_reinvention_candidate(&func.symbol, &func.path) {
            return None;
        }
        // Nearest *cross-file* neighbour (same-file matches are overloads /
        // adjacent helpers, a known false-alarm driver).
        let best = *self
            .index
            .nearest(query, 1, |e| e.path != func.path)
            .first()?;
        let best_entry = self.index.entry(best.entry_index);
        // A near-duplicate that keeps the *same name* in another file is almost
        // always a move/rename, not a reinvention — don't flag refactors.
        if eq_ignore_ascii_case(&best_entry.symbol, &func.symbol) {
            return None;
        }
        let cos = best.cosine;
        // Composition, not reinvention: if either function calls the other, the
        // new code *uses* the existing one rather than duplicating it. Common in
        // well-factored families (a `pointOnPolygon` that calls `pointOnLineSegment`
        // shares its vocabulary but is not a reinvention of it). Skipped for a
        // near-identical body (cos ≥ COMPOSITION_NEAR_DUP_SIM): at that similarity
        // the call is a name coincidence (a wrapper over a same-named *imported*
        // helper), not composition, and suppressing it would miss a true duplicate.
        if cos < COMPOSITION_NEAR_DUP_SIM
            && (func.callees.iter().any(|c| c == &best_entry.symbol)
                || best_entry.callees.iter().any(|c| c == &func.symbol))
        {
            return None;
        }
        let callee_jac = callee_jaccard(&func.callees, &best_entry.callees);
        let sub_jac = self.subtoken_jaccard(&func.subtokens, &best_entry.subtokens);
        // Both sides must clear the min-callee guard for the callee path to count.
        let both_callees =
            |min: usize| func.callees.len() >= min && best_entry.callees.len() >= min;

        let normal = cos >= NORMAL_SIMILARITY
            && ((both_callees(NORMAL_MIN_CALLEES) && callee_jac >= NORMAL_CALLEE_BAR)
                || sub_jac >= NORMAL_SUBTOKEN_BAR);
        let strong = cos >= STRONG_SIMILARITY
            && ((both_callees(STRONG_MIN_CALLEES) && callee_jac >= STRONG_CALLEE_BAR)
                || sub_jac >= STRONG_SUBTOKEN_BAR);
        // Rare-callee path: below the min-callee guard, a single shared *rare*
        // callee still confirms (a specific helper both functions call).
        let rare_callee = cos >= STRONG_SIMILARITY && self.shares_rare_callee(func, best_entry);
        if !normal && !strong && !rare_callee {
            return None;
        }
        Some(RedundantFinding {
            nearest_symbol: best_entry.symbol.clone(),
            nearest_path: best_entry.path.clone(),
            nearest_line: best_entry.line,
            similarity: cos,
        })
    }

    /// IDF-weighted Jaccard over two subtoken sets (shared *rare* subtokens count
    /// heavily, shared ubiquitous ones ~nothing).
    fn subtoken_jaccard(&self, a: &[String], b: &[String]) -> f32 {
        weighted_jaccard(a, b, &self.subtoken_idf, self.default_idf)
    }

    /// True if `func` and `entry` share at least one callee that is *rare* across
    /// the corpus (≤ `rare_callee_df` functions call it) — a discriminating shared
    /// helper, unlike overlap on one ubiquitous callee.
    fn shares_rare_callee(&self, func: &FunctionRef, entry: &IndexEntry) -> bool {
        let cand: BTreeSet<&str> = func.callees.iter().map(String::as_str).collect();
        entry.callees.iter().any(|c| {
            cand.contains(c.as_str())
                && self.callee_df.get(c).copied().unwrap_or(0) <= self.rare_callee_df
        })
    }
}

/// Plain Jaccard overlap of two callee sets (sorted, deduped). 0 when both empty.
/// Callees are *not* IDF-weighted: unlike identifier subtokens, a shared callee
/// is already a strong structural signal, and weighting cost recall for no net
/// false-alarm gain (the residual over-fire it targets is genuine duplication).
fn callee_jaccard(a: &[String], b: &[String]) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let sa: BTreeSet<&str> = a.iter().map(String::as_str).collect();
    let sb: BTreeSet<&str> = b.iter().map(String::as_str).collect();
    let union = sa.union(&sb).count();
    if union == 0 {
        0.0
    } else {
        sa.intersection(&sb).count() as f32 / union as f32
    }
}

/// Build the corpus IDF map for a family of per-function token sets.
fn corpus_idf<'a>(
    sets: impl Iterator<Item = &'a Vec<String>>,
    n_docs: f64,
) -> HashMap<String, f32> {
    let mut df: HashMap<&str, u32> = HashMap::new();
    for set in sets {
        for t in set {
            *df.entry(t.as_str()).or_insert(0) += 1;
        }
    }
    df.iter()
        .map(|(t, &c)| ((*t).to_string(), idf_of(c, n_docs)))
        .collect()
}

/// IDF-weighted Jaccard: shared *rare* tokens dominate, shared ubiquitous ones
/// carry ~nothing. A token unseen in the corpus gets the maximum (default)
/// weight. Both inputs are sorted + deduped.
fn weighted_jaccard(a: &[String], b: &[String], idf: &HashMap<String, f32>, default: f32) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let sa: BTreeSet<&str> = a.iter().map(String::as_str).collect();
    let sb: BTreeSet<&str> = b.iter().map(String::as_str).collect();
    let mut inter_w = 0.0f32;
    let mut union_w = 0.0f32;
    for t in sa.union(&sb) {
        let w = *idf.get(*t).unwrap_or(&default);
        union_w += w;
        if sa.contains(t) && sb.contains(t) {
            inter_w += w;
        }
    }
    if union_w > 0.0 {
        inter_w / union_w
    } else {
        0.0
    }
}

/// Smoothed inverse document frequency: `ln((N+1)/(df+1)) + 1`. Monotone-
/// decreasing in `df`, always ≥ 1, so a shared rare token outweighs a shared
/// common one without any hand-tuned stop-list.
fn idf_of(df: u32, n_docs: f64) -> f32 {
    (((n_docs + 1.0) / (df as f64 + 1.0)).ln() + 1.0) as f32
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

    fn entry_c(
        symbol: &str,
        path: &str,
        vec: Vec<f32>,
        callees: &[&str],
        subtokens: &[&str],
    ) -> IndexEntry {
        IndexEntry {
            symbol: symbol.into(),
            path: path.into(),
            line: 1,
            vec: unit(vec),
            callees: callees.iter().map(|s| s.to_string()).collect(),
            subtokens: subtokens.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn func_c(symbol: &str, path: &str, callees: &[&str], subtokens: &[&str]) -> FunctionRef {
        FunctionRef {
            symbol: symbol.into(),
            path: path.into(),
            line: 10,
            end_line: 15,
            text: String::new(),
            callees: callees.iter().map(|s| s.to_string()).collect(),
            subtokens: subtokens.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// `slugify` sits alone in one direction, a cluster of config code elsewhere.
    fn index() -> SemanticIndex {
        SemanticIndex {
            dim: 3,
            entries: vec![
                entry_c(
                    "slugify",
                    "src/utils/text.py",
                    vec![1.0, 0.0, 0.0],
                    &[],
                    &["slug", "normalize", "whitespace"],
                ),
                entry_c(
                    "parse_config",
                    "src/cfg.py",
                    vec![0.0, 1.0, 0.0],
                    &[],
                    &["config", "parse", "yaml"],
                ),
                entry_c(
                    "load_yaml",
                    "src/cfg.py",
                    vec![0.0, 0.9, 0.1],
                    &[],
                    &["yaml", "load", "stream"],
                ),
            ],
        }
    }

    #[test]
    fn fires_on_near_duplicate_via_subtokens() {
        let idx = index();
        let scorer = RedundantScorer::new(&idx);
        // Very close to `slugify` (cos ≈ 0.98) and shares its rare subtokens.
        let q = unit(vec![0.98, 0.02, 0.0]);
        let f = scorer
            .evaluate(
                &func_c(
                    "normalize_slug",
                    "src/api/handlers.py",
                    &[],
                    &["slug", "normalize", "whitespace"],
                ),
                &q,
            )
            .expect("subtoken-confirmed near-duplicate fires");
        assert_eq!(f.nearest_symbol, "slugify");
        assert!(f.similarity > 0.9);
    }

    #[test]
    fn does_not_fire_on_close_but_structurally_unrelated() {
        let idx = index();
        let scorer = RedundantScorer::new(&idx);
        // Close to `slugify` in embedding, but no shared callees and disjoint
        // subtokens → anisotropy near-match, not a reinvention.
        let q = unit(vec![0.98, 0.02, 0.0]);
        assert!(scorer
            .evaluate(
                &func_c(
                    "draw_widget",
                    "src/api/handlers.py",
                    &[],
                    &["draw", "widget", "canvas"]
                ),
                &q,
            )
            .is_none());
    }

    #[test]
    fn same_file_match_is_excluded() {
        let idx = index();
        let scorer = RedundantScorer::new(&idx);
        let q = unit(vec![0.98, 0.02, 0.0]);
        // Candidate lives in slugify's own file → its only near-dup is same-file.
        assert!(scorer
            .evaluate(
                &func_c(
                    "slug2",
                    "src/utils/text.py",
                    &[],
                    &["slug", "normalize", "whitespace"]
                ),
                &q,
            )
            .is_none());
    }

    #[test]
    fn same_name_is_treated_as_move_not_reinvention() {
        let idx = index();
        let scorer = RedundantScorer::new(&idx);
        let q = unit(vec![0.98, 0.02, 0.0]);
        // Same symbol name, different file → a move/rename, not a reinvention.
        assert!(scorer
            .evaluate(
                &func_c(
                    "slugify",
                    "src/api/handlers.py",
                    &[],
                    &["slug", "normalize", "whitespace"]
                ),
                &q,
            )
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

    /// A near-duplicate cluster where the candidate shares the original's callees
    /// but NOT its subtokens — the callee path must carry the fire on its own.
    fn callee_index() -> SemanticIndex {
        SemanticIndex {
            dim: 3,
            entries: vec![
                entry_c(
                    "format_price",
                    "src/money.py",
                    vec![1.0, 0.0, 0.0],
                    &["round", "currency", "symbol"],
                    &["price", "format"],
                ),
                entry_c(
                    "format_amount",
                    "src/money.py",
                    vec![0.99, 0.02, 0.0],
                    &["round", "currency"],
                    &["amount", "format"],
                ),
            ],
        }
    }

    #[test]
    fn composition_gate_suppresses_family_but_not_near_duplicate() {
        let idx = index();
        let scorer = RedundantScorer::new(&idx);
        // A moderately-distant match (cos ≈ 0.75, below the near-dup escape) that
        // calls the matched symbol `slugify` — composition (a larger function that
        // *uses* slugify), suppressed even though its subtokens would otherwise fire.
        let q_family = unit(vec![0.75, 0.66, 0.0]);
        assert!(
            scorer
                .evaluate(
                    &func_c(
                        "build_page_url",
                        "src/api/handlers.py",
                        &["slugify", "join"],
                        &["slug", "normalize", "whitespace"],
                    ),
                    &q_family,
                )
                .is_none(),
            "composition (cos<0.90) is suppressed"
        );
        // A near-identical body (cos ≈ 0.98) that happens to call a same-named
        // helper (e.g. an imported `slugify`) is a true duplicate — still fires.
        let q_dup = unit(vec![0.995, 0.02, 0.0]);
        let f = scorer
            .evaluate(
                &func_c(
                    "make_slug",
                    "src/api/handlers.py",
                    &["slugify", "strip"],
                    &["slug", "normalize", "whitespace"],
                ),
                &q_dup,
            )
            .expect("near-identical body fires despite same-named callee");
        assert_eq!(f.nearest_symbol, "slugify");
    }

    #[test]
    fn callee_overlap_fires_without_subtoken_overlap() {
        let idx = callee_index();
        let scorer = RedundantScorer::new(&idx);
        let q = unit(vec![0.995, 0.01, 0.0]); // cos ≈ 1.0 to format_price
        let f = scorer
            .evaluate(
                &func_c(
                    "render_price",
                    "src/ui/views.py",
                    &["round", "currency", "symbol"], // full callee overlap
                    &["render", "view"],              // disjoint subtokens
                ),
                &q,
            )
            .expect("callee overlap alone confirms the reinvention");
        assert_eq!(f.nearest_symbol, "format_price");
    }

    #[test]
    fn no_structural_overlap_does_not_fire() {
        // Same close embedding, but disjoint callees AND disjoint subtokens.
        let idx = callee_index();
        let scorer = RedundantScorer::new(&idx);
        let q = unit(vec![0.995, 0.01, 0.0]);
        assert!(scorer
            .evaluate(
                &func_c(
                    "compute_layout",
                    "src/ui/views.py",
                    &["measure", "wrap", "clamp"],
                    &["layout", "compute", "grid"],
                ),
                &q,
            )
            .is_none());
    }

    #[test]
    fn strong_tier_rescues_distant_match_with_high_subtoken_overlap() {
        let idx = index();
        let scorer = RedundantScorer::new(&idx);
        // cos ≈ 0.72 to slugify — below the normal 0.78 floor — but identical
        // rare subtokens. The strong tier rescues it; the normal tier would not.
        let q = unit(vec![0.72, 0.69, 0.0]);
        let f = scorer
            .evaluate(
                &func_c(
                    "make_slug",
                    "src/api/handlers.py",
                    &[],
                    &["slug", "normalize", "whitespace"],
                ),
                &q,
            )
            .expect("strong tier rescues a distant but structurally-identical match");
        assert_eq!(f.nearest_symbol, "slugify");
        assert!(
            f.similarity < NORMAL_SIMILARITY,
            "sim {} below normal floor",
            f.similarity
        );
        assert!(f.similarity >= STRONG_SIMILARITY);
    }

    #[test]
    fn distant_match_with_weak_overlap_does_not_fire() {
        let idx = index();
        let scorer = RedundantScorer::new(&idx);
        // Same cos ≈ 0.72, but only partial subtoken overlap (1 of 3 shared) →
        // below the strong bar, and too distant for the normal tier. No fire.
        let q = unit(vec![0.72, 0.69, 0.0]);
        assert!(scorer
            .evaluate(
                &func_c(
                    "make_slug",
                    "src/api/handlers.py",
                    &[],
                    &["slug", "trim", "pad"]
                ),
                &q,
            )
            .is_none());
    }
}
