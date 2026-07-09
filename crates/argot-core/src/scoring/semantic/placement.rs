//! F2 · placement — "this doesn't belong here".
//!
//! A function's *area* is the top-level package directory it lives in. If a
//! function's nearest existing neighbours nearly all live in a **different**
//! area than the one it's filed under — and its own area is under-represented
//! among them relative to what's normal for that area — the function is probably
//! misplaced (DB logic in a UI file, a parser in the HTTP layer). This is k-NN
//! area voting, which beat mean-to-centroid in exploration (AUC 0.71 → 0.88):
//! a function belongs where its *nearest* peers are, not where it's close on
//! average.
//!
//! Cross-cutting helpers legitimately span areas, so this stays advisory and
//! fires only on *strong* disagreement, calibrated against each area's own
//! typical locality (`index::calibrate_area_norms`).

use std::collections::{HashMap, HashSet};

use super::index::{FunctionRef, SemanticIndex};

/// Directory depth defining an "area". Depth 2 (the major package, `src/db` rather
/// than `src/db/models`) is both the more meaningful granularity for "this code is in
/// the wrong package" and materially higher recall: coarser areas let a transplanted
/// function's neighbours concentrate, so more genuine misplacements clear the gate —
/// at *lower* in-place over-fire than depth 3 (a function's own neighbours are more
/// often in-area). Swept over 10 corpora (scratchpad/f2sweep2.py).
pub const AREA_DEPTH: usize = 2;
/// Cross-file neighbours polled for the area vote.
const K_NEIGHBORS: usize = 10;
/// Minimum neighbours required to vote at all (too few → no signal, abstain).
const MIN_NEIGHBORS: usize = 5;
/// A candidate fires only if its in-area neighbour fraction is below this share
/// of its area's calibrated norm — "far below normal locality".
const MISPLACED_FACTOR: f32 = 0.3;
/// And at or below this absolute ceiling, so a high-locality area can't fire on
/// a merely slightly-below-norm function.
const ABS_IN_AREA_CEILING: f32 = 0.05;
/// And the function's neighbours must **concentrate** — the top two areas
/// together holding at least this share. Concentration (not single-area
/// plurality) is the right signal: a real misplacement's home can split across
/// sibling dirs (`core/downloader` + `downloadermiddlewares`), while a genuine
/// cross-cutting helper scatters across many areas. Tuned on rich + scrapy:
/// over-fire ~2.5% (scrapy) / 0.7% (rich) at ~41% transplant recall, down from
/// ~15% at the untuned thresholds; see `.scratch/semantic-layer/P5-tuning.md`.
/// Kept at 0.8 (the concentration that keeps in-place over-fire low); the recall
/// gain this era comes from the depth-2 area, which is also a large clean-commit
/// misplaced-FP *win* on cohesive monorepos (sibling packages collapse into one area
/// and stop reading as misplaced: laravel 34→1, homebrew 22→0, fmt 20%→3%). Some
/// corpora (hono/ink/jellyfin) still land below 85% transplant recall — a function
/// whose semantic peers are genuinely scattered across many packages cannot be shown
/// to be *mis*placed; an inherent limit of neighbour-based placement (and the
/// transplant metric is itself a lower bound, since generic utils relocate freely).
const MIN_TOP2_CONCENTRATION: f32 = 0.8;
/// Fallback belongs-norm for an area unseen at calibration.
const DEFAULT_NORM: f32 = 0.3;

/// The area (top-level package dir) a repo-relative path belongs to: its first
/// `depth` directory components. A top-level file has area `""` (the root).
pub fn area_of(path: &str, depth: usize) -> String {
    let comps: Vec<&str> = path.split('/').collect();
    if comps.len() <= 1 {
        return String::new(); // top-level file
    }
    let dirs = &comps[..comps.len() - 1]; // drop the filename
    dirs[..dirs.len().min(depth)].join("/")
}

/// The directory a repo-relative path lives in (everything before the last `/`),
/// or `""` for a top-level file. Finer than `area_of` — the exact location.
fn parent_dir(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

/// A fired placement finding: where the function looks like it belongs.
#[derive(Debug, Clone)]
pub struct MisplacedFinding {
    pub actual_area: String,
    /// The modal (most common) area among the nearest neighbours.
    pub neighbor_area: String,
    pub in_area_fraction: f32,
    pub expected_fraction: f32,
    /// Nearest peers (symbol, path, line) for evidence.
    pub peers: Vec<(String, String, usize)>,
}

/// A repo with fewer than this many distinct areas has too little architectural
/// separation for neighbour-based placement to mean anything — every function's
/// neighbours necessarily span most of the (tiny) area set, so any location reads
/// as "misplaced". Abstaining wholesale kills the false-fire storm on header-only
/// / single-package repos (fmt 2 areas → 20% F2 FP) with no cost on real repos,
/// which have many areas. Placement needs an architecture to judge against.
const MIN_DISTINCT_AREAS: usize = 4;

/// Scores diff-defined functions for architectural misplacement.
pub struct PlacementScorer<'a> {
    index: &'a SemanticIndex,
    /// Per-area calibrated "belongs" fraction (typical in-area neighbour share).
    area_norms: &'a HashMap<String, f32>,
    /// Every directory that held an indexed function at fit time (full dir path,
    /// not the coarse area). Placement can only judge a location with precedent.
    index_dirs: HashSet<String>,
    /// Distinct coarse areas (`AREA_DEPTH`) the fit repo has. Below
    /// [`MIN_DISTINCT_AREAS`] placement abstains entirely.
    n_areas: usize,
}

impl<'a> PlacementScorer<'a> {
    pub fn new(index: &'a SemanticIndex, area_norms: &'a HashMap<String, f32>) -> Self {
        let index_dirs: HashSet<String> = index
            .entries
            .iter()
            .map(|e| parent_dir(&e.path).to_string())
            .collect();
        let n_areas = index
            .entries
            .iter()
            .map(|e| area_of(&e.path, AREA_DEPTH))
            .collect::<HashSet<_>>()
            .len();
        Self {
            index,
            area_norms,
            index_dirs,
            n_areas,
        }
    }

    /// Evaluate one diff-defined function; `Some` when it looks misplaced.
    pub fn evaluate(&self, func: &FunctionRef, query: &[f32]) -> Option<MisplacedFinding> {
        // Too few areas → the repo has no architecture to judge placement against.
        if self.n_areas < MIN_DISTINCT_AREAS {
            return None;
        }
        if super::redundant::is_test_path(&func.path) {
            return None;
        }
        // A function in a directory that did NOT exist at fit time is *new*, not
        // *misplaced* — a brand-new feature/util dir has no same-area precedent, so
        // its functions inevitably embed near their functional peers elsewhere and
        // look "misplaced". Placement can only judge a location the repo already
        // has an opinion about. (A real misplacement adds code to an EXISTING wrong
        // dir, which is still judged.) This is the dominant clean-commit F2 FP.
        if !self.index_dirs.contains(parent_dir(&func.path)) {
            return None;
        }
        // Exclude only the function itself (a new diff function isn't in the
        // index, so this is a no-op at check) — same-file siblings are legitimate
        // area evidence, and keeping them makes placement work even in repos where
        // an area is a single file. F1, by contrast, must drop same-file matches.
        let neigh = self.index.nearest(query, K_NEIGHBORS, |e| {
            !(e.path == func.path && e.line == func.line)
        });
        if neigh.len() < MIN_NEIGHBORS {
            return None;
        }
        let actual = area_of(&func.path, AREA_DEPTH);
        let neighbor_areas: Vec<String> = neigh
            .iter()
            .map(|n| area_of(&self.index.entry(n.entry_index).path, AREA_DEPTH))
            .collect();
        let n = neighbor_areas.len() as f32;
        let in_area = neighbor_areas.iter().filter(|a| **a == actual).count() as f32 / n;
        let counts = area_counts(&neighbor_areas);
        let modal = counts[0].0.clone();
        let top2: usize = counts.iter().take(2).map(|(_, c)| *c).sum();
        let top2_frac = top2 as f32 / n;
        // Strong disagreement: peers concentrate elsewhere (top-2 areas), and this
        // function's own area is far below its calibrated locality norm.
        if modal == actual || top2_frac < MIN_TOP2_CONCENTRATION {
            return None;
        }
        let norm = self
            .area_norms
            .get(&actual)
            .copied()
            .unwrap_or(DEFAULT_NORM);
        if in_area > ABS_IN_AREA_CEILING || in_area >= norm * MISPLACED_FACTOR {
            return None;
        }
        let peers = neigh
            .iter()
            .take(3)
            .map(|n| {
                let e = self.index.entry(n.entry_index);
                (e.symbol.clone(), e.path.clone(), e.line)
            })
            .collect();
        Some(MisplacedFinding {
            actual_area: actual,
            neighbor_area: modal,
            in_area_fraction: in_area,
            expected_fraction: norm,
            peers,
        })
    }
}

/// Area → neighbour count, sorted by descending count then area name (so the
/// modal area and top-2 concentration are deterministic on ties).
fn area_counts(areas: &[String]) -> Vec<(String, usize)> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for a in areas {
        *counts.entry(a.as_str()).or_insert(0) += 1;
    }
    let mut v: Vec<(String, usize)> = counts
        .into_iter()
        .map(|(a, c)| (a.to_string(), c))
        .collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
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
            callees: Vec::new(),
            subtokens: Vec::new(),
        }
    }

    fn func(symbol: &str, path: &str) -> FunctionRef {
        FunctionRef {
            symbol: symbol.into(),
            path: path.into(),
            line: 10,
            end_line: 20,
            text: String::new(),
            callees: Vec::new(),
            subtokens: Vec::new(),
        }
    }

    #[test]
    fn area_of_takes_directory_prefix() {
        assert_eq!(area_of("src/db/models/user.py", 3), "src/db/models");
        assert_eq!(area_of("src/db/models/user.py", 2), "src/db");
        assert_eq!(area_of("src/api/handlers.py", 3), "src/api");
        assert_eq!(area_of("main.py", 3), "");
    }

    /// A dozen DB-flavoured functions in `src/db`, plus two UI ones in `src/ui`
    /// — enough that a DB-flavoured query's top-10 neighbours are all DB (the UI
    /// pair ranks below the cut), mirroring a real transplant's ~0 in-area share.
    /// Two extra areas (`src/net`, `src/util`) on unrelated axes give the index the
    /// ≥ [`MIN_DISTINCT_AREAS`] separation placement needs — without them the whole
    /// scorer abstains (see `abstains_when_too_few_areas`).
    fn index() -> SemanticIndex {
        let mut entries = Vec::new();
        for i in 0..12 {
            // db functions cluster along axis x
            entries.push(entry(
                &format!("db_{i}"),
                &format!("src/db/models/m{i}.py"),
                vec![1.0, 0.02 * i as f32, 0.0, 0.0],
            ));
        }
        entries.push(entry("render_a", "src/ui/views.py", vec![0.0, 1.0, 0.0, 0.0]));
        entries.push(entry("render_b", "src/ui/views.py", vec![0.0, 0.98, 0.05, 0.0]));
        // Two more areas on unrelated axes (z, w) — far from the db/ui query, so
        // they never enter a db query's top-k, but they make the repo ≥ 4 areas.
        for i in 0..4 {
            entries.push(entry(
                &format!("net_{i}"),
                &format!("src/net/http/h{i}.py"),
                vec![0.0, 0.0, 1.0, 0.02 * i as f32],
            ));
            entries.push(entry(
                &format!("util_{i}"),
                &format!("src/util/x{i}.py"),
                vec![0.0, 0.0, 0.02 * i as f32, 1.0],
            ));
        }
        SemanticIndex { dim: 4, entries }
    }

    fn norms() -> HashMap<String, f32> {
        // db area normally has high locality; ui too.
        HashMap::from([
            ("src/db/models".to_string(), 0.6f32),
            ("src/ui".to_string(), 0.6f32),
            ("src/net/http".to_string(), 0.6f32),
            ("src/util".to_string(), 0.6f32),
        ])
    }

    #[test]
    fn fires_on_db_function_filed_under_ui() {
        let idx = index();
        let norms = norms();
        let scorer = PlacementScorer::new(&idx, &norms);
        // A DB-flavoured function (axis x) but filed under src/ui.
        let q = unit(vec![1.0, 0.03, 0.0, 0.0]);
        let f = scorer
            .evaluate(&func("load_row", "src/ui/widgets.py"), &q)
            .expect("misplaced db-in-ui fires");
        assert_eq!(f.actual_area, "src/ui");
        assert_eq!(f.neighbor_area, "src/db"); // depth-2 area (major package)
        assert!(f.in_area_fraction < 0.2);
    }

    #[test]
    fn does_not_fire_on_in_place_function() {
        let idx = index();
        let norms = norms();
        let scorer = PlacementScorer::new(&idx, &norms);
        // A DB-flavoured function correctly filed under src/db.
        let q = unit(vec![1.0, 0.03, 0.0, 0.0]);
        assert!(scorer
            .evaluate(&func("load_row", "src/db/models/new.py"), &q)
            .is_none());
    }

    #[test]
    fn abstains_on_a_brand_new_directory() {
        let idx = index();
        let norms = norms();
        let scorer = PlacementScorer::new(&idx, &norms);
        // Same DB-flavoured query that fires when filed under the existing src/ui,
        // but here it lives in a directory the index has never seen. A new location
        // can't be judged misplaced — abstain (this is the dominant clean-commit FP:
        // a new feature/util dir whose functions embed near their peers elsewhere).
        let q = unit(vec![1.0, 0.03, 0.0, 0.0]);
        assert!(scorer
            .evaluate(&func("load_row", "src/brand_new_feature/impl.py"), &q)
            .is_none());
        // Control: the SAME function in an existing dir still fires.
        assert!(scorer
            .evaluate(&func("load_row", "src/ui/widgets.py"), &q)
            .is_some());
    }

    #[test]
    fn abstains_when_too_few_areas() {
        // A repo with only 2 areas (src/db, src/ui) — below MIN_DISTINCT_AREAS.
        // Even a clearly-misplaced db-in-ui function abstains: with so little
        // architectural separation the placement signal is noise (fmt's 20% F2 FP).
        let mut entries = Vec::new();
        for i in 0..12 {
            entries.push(entry(
                &format!("db_{i}"),
                &format!("src/db/models/m{i}.py"),
                vec![1.0, 0.02 * i as f32, 0.0, 0.0],
            ));
        }
        entries.push(entry("render_a", "src/ui/views.py", vec![0.0, 1.0, 0.0, 0.0]));
        entries.push(entry("render_b", "src/ui/views.py", vec![0.0, 0.98, 0.05, 0.0]));
        let idx = SemanticIndex { dim: 4, entries };
        let norms = norms();
        let scorer = PlacementScorer::new(&idx, &norms);
        let q = unit(vec![1.0, 0.03, 0.0, 0.0]);
        assert!(scorer
            .evaluate(&func("load_row", "src/ui/widgets.py"), &q)
            .is_none());
    }

    #[test]
    fn parent_dir_extracts_directory() {
        assert_eq!(parent_dir("src/ui/widgets.py"), "src/ui");
        assert_eq!(parent_dir("main.py"), "");
    }
}
