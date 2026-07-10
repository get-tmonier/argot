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

use serde::{Deserialize, Serialize};

use super::index::{FunctionRef, IndexEntry, SemanticIndex};
use super::placement::parent_dir;

/// Normal tier — a close embedding match with *moderate* structural agreement.
const NORMAL_SIMILARITY: f32 = 0.78;
const NORMAL_SUBTOKEN_BAR: f32 = 0.40;
const NORMAL_CALLEE_BAR: f32 = 0.12;
/// The callee path needs both sides to have at least this many callees — a
/// 1-callee fn matching a 1-callee fn is 100% overlap by luck, not evidence.
/// Exception: when both sides have EXACTLY ONE callee and it is the *same*
/// one (identical single-callee sets at normal-tier cosine), the overlap is
/// no longer luck — both functions are built around the same specific helper.
/// Recovers small util reimplementations (geometry helpers wrapping one
/// primitive) at zero measured clean-commit cost (all-gates sweep).
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
/// already have this". Applied UNCONDITIONALLY at 5 lines (clean-commit stub
/// false fires — `assertTrue` overloads, `isDone` accessors — are 3–4 lines and
/// fired through the strong-overlap paths the old weak-overlap-only floor never
/// gated; the shortest genuine planted reimplementation across 31 corpora is
/// 5 lines). The stricter 6-line floor still applies under weak overlap.
const MIN_BODY_LINES_ANY: usize = 5;
const MIN_REINVENTION_BODY_LINES: usize = 6;
/// A same-directory near-match must also *stand out* from the rest of the
/// neighbourhood: co-located wrapper/API families (`cf_h3_proxy_*` next to
/// `cf_h2_proxy_*`, protocol-variant ports) are the dominant clean-commit false
/// fire on flat-layout repos, and they sit in a crowd of near-equal neighbours
/// (margin cos₁−cos₂ ≈ 0.01–0.06), while a genuine reinvention matches ONE
/// original (median margin 0.17). Same-file exclusion logic, one level up, as a
/// margin bar instead of a hard exclusion.
const SAME_DIR_MIN_MARGIN: f32 = 0.10;
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
/// Tightened 7→5 together with the exemption change below: locale/provider
/// families (`romanized_name` across per-locale providers) sat at 4–6 neighbours
/// with callee_jac 1.0 on *generic* shared helpers and slipped the old
/// callee-strength exemption; genuine planted reimplantations sit at 1–3.
const VERY_DENSE_NEIGHBORS: usize = 5;
const CLUSTER_COSINE_BAND: f32 = 0.05;
/// Cross-file neighbours retrieved to measure family density.
const NEIGHBORHOOD: usize = 10;

// --- conservative mode (fit-time self-calibration) -----------------------------
//
/// Some repos practice *systematic parallel implementation* — per-entity modules
/// (checkout/order webhooks), protocol-variant ports — where every new function
/// legitimately near-duplicates an existing sibling. There the standard rule
/// over-fires on clean commits. At fit time a **mini-replay** estimates exactly
/// that: the fraction of functions ADDED in the recent window (function-level,
/// via git tree diff + old-version symbol parsing — no extra embedding) that the
/// rule would flag against the pre-window code. Above the bar, the repo's F1
/// switches to conservative mode: fires must be *unambiguous* (close match that
/// stands out from the crowd).
const CONSERVATIVE_EST_BAR: f32 = 0.09;
/// Minimum recently-added functions for the estimate to mean anything — at a
/// ~10% rate, fewer observations put the binomial CI wider than the decision
/// band (junit5: 7/62 recents ≈ 11% estimate against a measured 1.6%/hunk
/// clean-commit rate). Low-churn repos don't exhibit the systematic-parallel
/// pattern at scale anyway.
const CONSERVATIVE_MIN_RECENT: usize = 100;
/// Recently-added functions sampled for the estimate (stride cap).
const CONSERVATIVE_MAX_SAMPLE: usize = 400;
/// Conservative-mode extra gates: the match must be close…
const CONSERVATIVE_MIN_COSINE: f32 = 0.85;
/// …and stand out from the second-nearest neighbour.
const CONSERVATIVE_MIN_MARGIN: f32 = 0.05;
/// Mirror guard: in a corpus where this share of functions' two nearest
/// cross-file neighbours carry the SAME symbol (a maintained mirror tree —
/// guava/ + android/ at 48%), a query matching a mirrored function sees both
/// twins at the top and its margin collapses to ~0 whatever the evidence, so
/// the conservative margin gate would blind the whole sense (guava planted
/// recall 94%→0%). Conservative mode is only usable below this rate.
const CONSERVATIVE_MAX_TWIN_RATE: f32 = 0.35;
/// Index functions sampled for the twin-rate measurement.
const TWIN_RATE_SAMPLE: usize = 300;

/// Fit-time self-calibrated reinvention configuration, stored per language in
/// the semantic artifact.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReinventionConfig {
    /// Apply the conservative extra gates at check time.
    pub conservative: bool,
    /// Diagnostics: the mini-replay estimate (fires / recently-added fns) and
    /// how many recently-added functions it saw.
    pub est_fire_rate: f32,
    pub est_recent: usize,
}

/// Self-calibrate F1 on a fresh index. `recent[i]` marks entries added within
/// the recent history window (computed by the fit flow via git). The estimator
/// evaluates each recent entry against an index of only the NON-recent entries
/// — approximating "new code checked against the old tree" with zero extra
/// embedding work.
pub fn calibrate_reinvention(index: &SemanticIndex, recent: &[bool]) -> ReinventionConfig {
    let recents: Vec<usize> = (0..index.len()).filter(|&i| recent[i]).collect();
    let old_entries: Vec<IndexEntry> = index
        .entries
        .iter()
        .zip(recent)
        .filter(|(_, r)| !**r)
        .map(|(e, _)| e.clone())
        .collect();
    let mut cfg = ReinventionConfig {
        conservative: false,
        est_fire_rate: 0.0,
        est_recent: recents.len(),
    };
    if recents.len() < CONSERVATIVE_MIN_RECENT || old_entries.len() < CONSERVATIVE_MIN_RECENT {
        return cfg;
    }
    let old_index = SemanticIndex {
        dim: index.dim,
        entries: old_entries,
    };
    let scorer = RedundantScorer::new(&old_index, &ReinventionConfig::default());
    let step = recents.len().div_ceil(CONSERVATIVE_MAX_SAMPLE).max(1);
    let mut fires = 0usize;
    let mut evaluated = 0usize;
    for &i in recents.iter().step_by(step) {
        let e = index.entry(i);
        // Body text is not stored in the index; use a substantial placeholder so
        // the substance floors pass (the estimator measures the structural rule).
        let func = FunctionRef {
            symbol: e.symbol.clone(),
            path: e.path.clone(),
            line: e.line,
            end_line: e.line + 10,
            text: "x\n".repeat(12),
            callees: e.callees.clone(),
            subtokens: e.subtokens.clone(),
        };
        evaluated += 1;
        if scorer.evaluate(&func, &e.vec).is_some() {
            fires += 1;
        }
    }
    if evaluated > 0 {
        cfg.est_fire_rate = fires as f32 / evaluated as f32;
        cfg.conservative = cfg.est_fire_rate >= CONSERVATIVE_EST_BAR
            && twin_rate(index) < CONSERVATIVE_MAX_TWIN_RATE;
    }
    cfg
}

/// Share of (sampled) index functions whose two nearest cross-file neighbours
/// carry the same symbol — the mirror-tree signature (see
/// [`CONSERVATIVE_MAX_TWIN_RATE`]).
fn twin_rate(index: &SemanticIndex) -> f32 {
    let n = index.len();
    if n < 50 {
        return 0.0;
    }
    let step = n.div_ceil(TWIN_RATE_SAMPLE).max(1);
    let mut twin = 0usize;
    let mut total = 0usize;
    for qi in (0..n).step_by(step) {
        let e = index.entry(qi);
        let top = index.nearest(&e.vec, 2, |o| o.path != e.path);
        if top.len() < 2 {
            continue;
        }
        total += 1;
        let a = &index.entry(top[0].entry_index).symbol;
        let b = &index.entry(top[1].entry_index).symbol;
        if a.len() == b.len() && a.eq_ignore_ascii_case(b) {
            twin += 1;
        }
    }
    if total == 0 {
        0.0
    } else {
        twin as f32 / total as f32
    }
}

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
    /// Fit-time self-calibrated mode (see [`ReinventionConfig`]).
    conservative: bool,
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
    pub fn new(index: &'a SemanticIndex, cfg: &ReinventionConfig) -> Self {
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
            conservative: cfg.conservative,
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
        // Identical single-callee sets: both built around the same one helper.
        let single_shared_callee =
            func.callees.len() == 1 && best_entry.callees.len() == 1 && callee_jac >= 1.0;

        let normal = cos >= NORMAL_SIMILARITY
            && ((both_callees(NORMAL_MIN_CALLEES) && callee_jac >= NORMAL_CALLEE_BAR)
                || sub_jac >= NORMAL_SUBTOKEN_BAR
                || single_shared_callee);
        let strong = cos >= STRONG_SIMILARITY
            && ((both_callees(STRONG_MIN_CALLEES) && callee_jac >= STRONG_CALLEE_BAR)
                || sub_jac >= STRONG_SUBTOKEN_BAR);
        // Rare-callee path: below the min-callee guard, a single shared *rare*
        // callee still confirms (a specific helper both functions call).
        let rare_callee = self.shares_rare_callee(func, best_entry);
        if !normal && !strong && !(cos >= STRONG_SIMILARITY && rare_callee) {
            return None;
        }
        // Substance floor (unconditional): stubs and accessors are never a
        // meaningful "you already have this", however strong the overlap.
        if func.text.lines().count() < MIN_BODY_LINES_ANY {
            return None;
        }
        let margin = neighbors
            .get(1)
            .map(|n2| cos - n2.cosine)
            .unwrap_or(f32::MAX);
        // Same-directory matches must stand out from the neighbourhood (see
        // SAME_DIR_MIN_MARGIN): a co-located protocol-variant / wrapper family
        // member sits in a crowd; a genuine reinvention matches one original.
        if parent_dir(&func.path) == parent_dir(&best_entry.path) && margin < SAME_DIR_MIN_MARGIN {
            return None;
        }
        // Conservative mode (fit-time self-calibrated, see ReinventionConfig):
        // a repo shown to practice systematic parallel implementation only gets
        // unambiguous findings — a close match that stands out from the crowd.
        if self.conservative && (cos < CONSERVATIVE_MIN_COSINE || margin < CONSERVATIVE_MIN_MARGIN)
        {
            return None;
        }
        // Sibling / wrapper filters — reject the non-reinvention shapes that embed
        // close but dominate clean-commit false fires (see the constants + evidence).
        let match_df = self.symbol_df.get(&best_entry.symbol).copied().unwrap_or(1);
        // Family tiers (unconditional). A candidate that matches a *very* dense cluster
        // (5+ near-identical cross-file neighbours) or a *very* common name (defined
        // 20+ times across the repo) is a family member — a new command handler /
        // provider method / assertion overload, not a unique reinvention. It fires
        // anyway only when it shares a *rare* callee with its match: a genuine
        // reimplementation of one specific member calls the same distinctive helper;
        // a parallel sibling shares only the family's generic ones (callee-Jaccard
        // alone proved too weak an exemption — per-locale provider families overlap
        // 100% on generic helpers).
        if !rare_callee && (n_near >= VERY_DENSE_NEIGHBORS || match_df >= VERY_FAMILIAR_SYMBOL_DF) {
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
        let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
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
        let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
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
        let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
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
        let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
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
        let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
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
        let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
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
        let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
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
        let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
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
    /// one of them is a family member, not a reinvention; one that shares a member's
    /// *rare* helper is a genuine reimplementation of that member and still fires.
    #[test]
    fn sibling_in_a_dense_family_is_filtered_unless_it_shares_a_rare_helper() {
        let mut entries = Vec::new();
        for i in 0..8 {
            // One member owns a distinctive helper (`rotl64`, df=1 → rare); the
            // family's shared helpers (`base`, `emit`, df=8) are generic.
            let callees: &[&str] = if i == 0 {
                &["base", "emit", "rotl64"]
            } else {
                &["base", "emit"]
            };
            entries.push(entry_c(
                "handle",
                &format!("src/mod{i}/h.rs"),
                vec![1.0, 0.004 * i as f32, 0.0],
                callees,
                &["handle", "event"],
            ));
        }
        let idx = SemanticIndex { dim: 3, entries };
        let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
        // Nearest member is mod0 — the one with the distinctive helper.
        let q = unit(vec![1.0, 0.0, 0.0]);
        // Loosely-resembling new sibling: fires the callee tier (shares `base`) but
        // weak overlap in a very dense neighbourhood → filtered as a family member.
        let weak = func_c(
            "route",
            "src/new/h.rs",
            &["base", "x", "y"],
            &["route", "path"],
        );
        assert!(scorer.evaluate(&weak, &q).is_none());
        // Full generic-callee overlap is NOT an exemption any more (per-locale
        // provider families overlap 100% on generic helpers) — still filtered.
        let generic = func_c(
            "route",
            "src/new/h.rs",
            &["base", "emit"],
            &["route", "path"],
        );
        assert!(scorer.evaluate(&generic, &q).is_none());
        // Sharing the one member's RARE helper is genuine reimplementation
        // evidence → exempt from the family filter, fires.
        let genuine = func_c(
            "route",
            "src/new/h.rs",
            &["base", "emit", "rotl64"],
            &["route", "path"],
        );
        assert!(scorer.evaluate(&genuine, &q).is_some());
    }

    /// Same-directory near-matches need to stand out from the neighbourhood: a
    /// co-located protocol-variant family member (tiny cos₁−cos₂ margin) is
    /// filtered; the same candidate in another directory fires.
    #[test]
    fn same_dir_match_requires_margin() {
        let idx = SemanticIndex {
            dim: 3,
            entries: vec![
                entry_c(
                    "h2_go_state",
                    "lib/proxy.py",
                    vec![1.0, 0.0, 0.0],
                    &[],
                    &["tunnel", "go", "state"],
                ),
                // A second neighbour almost as close → margin ≈ 0.
                entry_c(
                    "h2_reset",
                    "lib/other.py",
                    vec![0.999, 0.03, 0.0],
                    &[],
                    &["tunnel", "reset"],
                ),
            ],
        };
        let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
        let q = unit(vec![1.0, 0.01, 0.0]);
        // Same dir as the match (lib/) and margin ~0 → filtered.
        let same_dir = func_c(
            "h3_go_state",
            "lib/proxy_h3.py",
            &[],
            &["tunnel", "go", "state"],
        );
        assert!(scorer.evaluate(&same_dir, &q).is_none());
        // Identical candidate in a different directory → fires.
        let elsewhere = func_c(
            "h3_go_state",
            "src/http/proxy_h3.py",
            &[],
            &["tunnel", "go", "state"],
        );
        assert!(scorer.evaluate(&elsewhere, &q).is_some());
    }

    /// The substance floor applies unconditionally: a 4-line stub never fires,
    /// even with perfect structural overlap.
    #[test]
    fn short_stub_never_fires_even_with_strong_overlap() {
        let idx = index();
        let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
        let q = unit(vec![0.98, 0.02, 0.0]);
        let stub = FunctionRef {
            symbol: "normalize_slug".into(),
            path: "src/api/handlers.py".into(),
            line: 1,
            end_line: 4,
            text: "def normalize_slug(s):\n    a = s.strip()\n    b = a.lower()\n    return b"
                .into(),
            callees: vec![],
            subtokens: vec!["slug".into(), "normalize".into(), "whitespace".into()],
        };
        assert!(scorer.evaluate(&stub, &q).is_none());
    }

    /// Both sides built around the same single helper: the identical-single-callee
    /// path confirms at normal-tier cosine (util reimplementations wrapping one
    /// primitive have exactly one callee and generic subtokens).
    #[test]
    fn identical_single_callee_confirms_at_normal_cosine() {
        let idx = SemanticIndex {
            dim: 3,
            entries: vec![
                entry_c(
                    "point_rotate",
                    "src/math/point.py",
                    vec![1.0, 0.0, 0.0],
                    &["rotate_rads"],
                    &["point", "rotate"],
                ),
                entry_c(
                    "unrelated",
                    "src/cfg.py",
                    vec![0.0, 1.0, 0.0],
                    &["load"],
                    &["config"],
                ),
            ],
        };
        let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
        let q = unit(vec![1.0, 0.05, 0.0]);
        let f = scorer
            .evaluate(
                &func_c(
                    "rotate_point",
                    "src/geometry/util.py",
                    &["rotate_rads"],    // same single callee
                    &["vector", "spin"], // disjoint subtokens
                ),
                &q,
            )
            .expect("identical single callee confirms");
        assert_eq!(f.nearest_symbol, "point_rotate");
    }

    /// A thin delegator/wrapper is too short to be a meaningful reinvention when it
    /// only loosely resembles its match.
    #[test]
    fn short_wrapper_is_filtered_when_overlap_is_weak() {
        let idx = callee_index(); // format_price / format_amount
        let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
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
