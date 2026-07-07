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

use std::collections::HashMap;

use super::index::{FunctionRef, SemanticIndex};

/// Directory depth defining an "area" (exploration slightly favoured 3 over 2).
pub const AREA_DEPTH: usize = 3;
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

/// Scores diff-defined functions for architectural misplacement.
pub struct PlacementScorer<'a> {
    index: &'a SemanticIndex,
    /// Per-area calibrated "belongs" fraction (typical in-area neighbour share).
    area_norms: &'a HashMap<String, f32>,
}

impl<'a> PlacementScorer<'a> {
    pub fn new(index: &'a SemanticIndex, area_norms: &'a HashMap<String, f32>) -> Self {
        Self { index, area_norms }
    }

    /// Evaluate one diff-defined function; `Some` when it looks misplaced.
    pub fn evaluate(&self, func: &FunctionRef, query: &[f32]) -> Option<MisplacedFinding> {
        if super::redundant::is_test_path(&func.path) {
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
        }
    }

    fn func(symbol: &str, path: &str) -> FunctionRef {
        FunctionRef {
            symbol: symbol.into(),
            path: path.into(),
            line: 10,
            end_line: 20,
            text: String::new(),
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
    fn index() -> SemanticIndex {
        let mut entries = Vec::new();
        for i in 0..12 {
            // db functions cluster along axis x
            entries.push(entry(
                &format!("db_{i}"),
                &format!("src/db/models/m{i}.py"),
                vec![1.0, 0.02 * i as f32, 0.0],
            ));
        }
        entries.push(entry("render_a", "src/ui/views.py", vec![0.0, 1.0, 0.0]));
        entries.push(entry("render_b", "src/ui/views.py", vec![0.0, 0.98, 0.05]));
        SemanticIndex { dim: 3, entries }
    }

    fn norms() -> HashMap<String, f32> {
        // db area normally has high locality; ui too.
        HashMap::from([
            ("src/db/models".to_string(), 0.6f32),
            ("src/ui".to_string(), 0.6f32),
        ])
    }

    #[test]
    fn fires_on_db_function_filed_under_ui() {
        let idx = index();
        let norms = norms();
        let scorer = PlacementScorer::new(&idx, &norms);
        // A DB-flavoured function (axis x) but filed under src/ui.
        let q = unit(vec![1.0, 0.03, 0.0]);
        let f = scorer
            .evaluate(&func("load_row", "src/ui/widgets.py"), &q)
            .expect("misplaced db-in-ui fires");
        assert_eq!(f.actual_area, "src/ui");
        assert_eq!(f.neighbor_area, "src/db/models");
        assert!(f.in_area_fraction < 0.2);
    }

    #[test]
    fn does_not_fire_on_in_place_function() {
        let idx = index();
        let norms = norms();
        let scorer = PlacementScorer::new(&idx, &norms);
        // A DB-flavoured function correctly filed under src/db.
        let q = unit(vec![1.0, 0.03, 0.0]);
        assert!(scorer
            .evaluate(&func("load_row", "src/db/models/new.py"), &q)
            .is_none());
    }
}
