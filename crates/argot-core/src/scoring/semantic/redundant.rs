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
/// Tightened from 0.012 after clean-commit FP labelling: at 1.2% the "rare" band
/// still admitted borderline framework utilities (`deferred_from_coro`, df ~1%),
/// which drove ~40% of one corpus's clean-commit false fires on a single shared
/// helper; a genuinely distinctive helper sits far below that. Recall-neutral (the
/// genuine reimpls this path carries share helpers an order of magnitude rarer).
const RARE_CALLEE_DF_FRACTION: f64 = 0.004;
const RARE_CALLEE_DF_FLOOR: u32 = 4;

/// The lowest cosine at which a reinvention can fire (the strong-tier floor).
/// Exposed so the check flow can report it as the finding's informational
/// "threshold".
pub(crate) const MIN_SIMILARITY_TO_FIRE: f32 = STRONG_SIMILARITY;

// --- sibling / wrapper filters -------------------------------------------------
// A reinvention is *substantive, unique* code. Three shapes embed close to an
// existing function but are not reinventions, and dominate the clean-commit false
// fires on library/framework corpora (see docs/research/evidence/semantic-f1-*).
//
/// Substance floor: a function this short is a thin wrapper / delegator / accessor
/// (`fn lower(&self){ self.floor() }`). Matching one is never a meaningful "you
/// already have this". Genuine reimplementations are longer (the shortest planted
/// reinvention across 31 corpora is ~7 lines).
const MIN_REINVENTION_BODY_LINES: usize = 6;
/// A symbol name defined at least this many times across the repo is an *interface
/// / family method* (`on_send` in a linter's cops, `ReadMetadata` across providers)
/// — you cannot reinvent a method every sibling class already implements. A genuine
/// reinvention target is defined ~once. Applied under the weak-overlap guard.
const FAMILY_SYMBOL_DF: u32 = 5;
/// A name this common (an assertion overload defined 100+ times, a `parse` / `read`
/// defined dozens of times) is *unconditionally* a family method — filtered like a
/// very-dense cluster, subject to the same exact-helper (callee) exemption.
const VERY_FAMILIAR_SYMBOL_DF: u32 = 20;
/// Embedding-cluster density: this many of the top-10 cross-file neighbours sitting
/// within [`CLUSTER_COSINE_BAND`] of the nearest marks a *dense family* of
/// near-identical siblings (per-entity resolvers, per-codec handlers). A unique
/// reinvention matches ONE original, so its neighbours drop off. Applied only under
/// the weak-overlap guard.
const DENSE_CLUSTER_NEIGHBORS: usize = 3;
/// A *very* dense cluster (this many near-identical neighbours) is a family member
/// regardless of overlap strength — even a strong match to one member is just "the
/// next sibling in a large family", not a reinvention. Applied unconditionally.
/// Set one above the densest genuine reinvention observed by subtoken/vocabulary
/// alone (a `password` generator at 6) so it never suppresses a real reimplementation.
const VERY_DENSE_NEIGHBORS: usize = 7;
/// …and a very-dense candidate that reuses its match's *exact* helpers this strongly
/// is a genuine reimplementation of one specific member, not just the next sibling.
const VERY_DENSE_CALLEE_EXEMPT: f32 = 0.50;
const CLUSTER_COSINE_BAND: f32 = 0.05;
/// Cross-file neighbours retrieved to measure family density.
const NEIGHBORHOOD: usize = 10;

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
    /// How many index functions share each symbol name — an interface/family
    /// method (`on_send`, `ReadMetadata`) recurs across the repo; a unique
    /// function is defined ~once.
    symbol_df: HashMap<String, u32>,
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
        let mut symbol_df: HashMap<String, u32> = HashMap::new();
        for e in &index.entries {
            *symbol_df.entry(e.symbol.clone()).or_insert(0) += 1;
        }
        Self {
            index,
            subtoken_idf,
            default_idf: idf_of(0, n_docs),
            callee_df,
            rare_callee_df,
            symbol_df,
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
        let neighbors = self
            .index
            .nearest(query, NEIGHBORHOOD, |e| e.path != func.path);
        let best = *neighbors.first()?;
        let best_entry = self.index.entry(best.entry_index);
        // Embedding-family density: how many of the top-`NEIGHBORHOOD` cross-file
        // neighbours sit within [`CLUSTER_COSINE_BAND`] cosine of the nearest. A
        // unique reinvention matches ONE original (the rest drop off); a sibling
        // interface method sits in a crowd of near-equal neighbours.
        let n_near = neighbors
            .iter()
            .filter(|n| n.cosine >= best.cosine - CLUSTER_COSINE_BAND)
            .count();
        // A near-duplicate that keeps the *same name* in another file is almost
        // always a move/rename, not a reinvention — don't flag refactors.
        if eq_ignore_ascii_case(&best_entry.symbol, &func.symbol) {
            return None;
        }
        // Composition, not reinvention: if either function calls the other, the
        // new code *uses* the existing one rather than duplicating it. Common in
        // well-factored families (a `pointOnPolygon` that calls `pointOnLineSegment`
        // shares its vocabulary but is not a reinvention of it).
        if func.callees.iter().any(|c| c == &best_entry.symbol)
            || best_entry.callees.iter().any(|c| c == &func.symbol)
        {
            return None;
        }
        let cos = best.cosine;
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
        // Sibling / wrapper filters — reject the non-reinvention shapes that embed
        // close but dominate clean-commit false fires (see the constants + evidence).
        let match_df = self.symbol_df.get(&best_entry.symbol).copied().unwrap_or(1);
        // Family tiers (unconditional). A candidate that matches a *very* dense cluster
        // (7+ near-identical cross-file neighbours) or a *very* common name (defined
        // 20+ times across the repo) is a family member — a new command handler /
        // provider method / assertion overload, not a unique reinvention. It fires
        // anyway only when it reuses its match's *exact* helpers (callee ≥ 0.50): a
        // genuine reimplementation of one specific member (a bit-rotation that calls
        // the same primitives) does that; a parallel sibling shares only a fraction.
        if callee_jac < VERY_DENSE_CALLEE_EXEMPT
            && (n_near >= VERY_DENSE_NEIGHBORS || match_df >= VERY_FAMILIAR_SYMBOL_DF)
        {
            return None;
        }
        // The remaining filters apply only when the structural overlap with the match
        // is *weak*: a strong / near-identical match to a unique original is a genuine
        // reimplementation even if it is short, a common name, or in a moderate family.
        let weak_overlap = callee_jac < STRONG_CALLEE_BAR && sub_jac < NORMAL_SUBTOKEN_BAR;
        if weak_overlap {
            if func.text.lines().count() < MIN_REINVENTION_BODY_LINES {
                return None; // thin wrapper / accessor
            }
            if match_df >= FAMILY_SYMBOL_DF {
                return None; // matched an interface / family method
            }
            if n_near >= DENSE_CLUSTER_NEIGHBORS {
                return None; // one of many near-identical siblings, only loosely resembled
            }
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
        // A ≥6-line body so the substance filter (MIN_REINVENTION_BODY_LINES) treats
        // these as real functions; the tests exercise the structural fire logic, not
        // the wrapper filter (which has its own tests).
        let text = format!("fn {symbol}() {{\n    let a = 1;\n    let b = 2;\n    let c = a + b;\n    let d = c * 2;\n    d\n}}");
        FunctionRef {
            symbol: symbol.into(),
            path: path.into(),
            line: 10,
            end_line: 16,
            text,
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

    /// A large family of near-identical functions (an interface implemented across
    /// many files) all embed together. A new sibling that only *loosely* resembles
    /// one of them is a family member, not a reinvention; one that reuses a member's
    /// exact helpers is a genuine reimplementation and still fires.
    #[test]
    fn sibling_in_a_dense_family_is_filtered_unless_it_reuses_exact_helpers() {
        let mut entries = Vec::new();
        for i in 0..8 {
            entries.push(entry_c(
                "handle",
                &format!("src/mod{i}/h.rs"),
                vec![1.0, 0.004 * i as f32, 0.0],
                &["base", "emit"],
                &["handle", "event"],
            ));
        }
        let idx = SemanticIndex { dim: 3, entries };
        let scorer = RedundantScorer::new(&idx);
        let q = unit(vec![0.999, 0.01, 0.0]);
        // Loosely-resembling new sibling: fires the callee tier (shares `base`) but
        // weak overlap in a very dense neighbourhood → filtered as a family member.
        let weak = func_c(
            "route",
            "src/new/h.rs",
            &["base", "x", "y"],
            &["route", "path"],
        );
        assert!(scorer.evaluate(&weak, &q).is_none());
        // Genuine reimplementation of one member: reuses its exact helpers (high
        // callee overlap) → exempt from the family filter, fires.
        let genuine = func_c(
            "route",
            "src/new/h.rs",
            &["base", "emit"],
            &["route", "path"],
        );
        assert!(scorer.evaluate(&genuine, &q).is_some());
    }

    /// A thin delegator/wrapper is too short to be a meaningful reinvention when it
    /// only loosely resembles its match.
    #[test]
    fn short_wrapper_is_filtered_when_overlap_is_weak() {
        let idx = callee_index(); // format_price / format_amount
        let scorer = RedundantScorer::new(&idx);
        let q = unit(vec![0.995, 0.01, 0.0]); // ≈1.0 to format_price
        let short = FunctionRef {
            symbol: "show".into(),
            path: "src/ui.rs".into(),
            line: 1,
            end_line: 3,
            text: "fn show() {\n    round()\n}".into(), // 3 lines, below the substance floor
            callees: vec!["round".into(), "x".into(), "y".into()], // callee_jac ≈0.20 (weak)
            subtokens: vec!["show".into()],
        };
        assert!(scorer.evaluate(&short, &q).is_none());
    }
}
