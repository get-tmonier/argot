//! F2 · placement — "this doesn't belong here".
//!
//! A function's *area* is found adaptively: walking down from the repo root, a
//! directory holding more than half of its parent's functions is a *container*
//! (`src/`, `src/Illuminate/`) — the walk descends into it; the first
//! non-dominant directory is the area. Areas therefore sit at mixed depths and
//! match each repo's real package granularity (fixes the fixed-depth failure
//! where `src/Composer` swallowed a whole corpus into one area).
//!
//! Areas that are semantically *entangled* — a large share of one area's
//! nearest neighbours land in the other (`src/` vs `include/fmt` in a
//! header-only library) — are merged: placement between them is not judgeable.
//!
//! The fire rule is a k-NN area vote: a function is misplaced when the modal
//! area of its nearest neighbours differs from the area it's filed under AND
//! at most `z` of those neighbours share its area. `(merge τ, k, z)` are
//! **self-calibrated at fit time** per repo: a transplant simulation (every
//! sampled function claimed into every foreign area) plus an in-place
//! over-fire measurement pick the config with the highest simulated recall
//! under a hard over-fire cap — and when no config reaches usable recall, the
//! repo's placement sense is *disabled* (a repo whose areas the embedding
//! cannot separate — flat single-package layouts, header-only libraries —
//! gets a clean abstain, not noise).

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::index::{FunctionRef, SemanticIndex};

/// A directory holding more than this share of its parent's functions is a
/// container, not an area — the adaptive walk descends into it.
const MAX_CONTAINER_FRAC: f64 = 0.5;
/// …and so is a directory holding more than this share of ALL functions,
/// whatever its parent share: a thousands-of-functions subtree is never a leaf
/// area (mirror layouts like guava/ + android/ split ~50/50, so neither clears
/// the parent-share test, yet each contains the real packages).
const ABS_CONTAINER_FRAC: f64 = 0.25;
/// An area smaller than this merges up into its parent container.
const MIN_AREA_FNS: usize = 8;
/// Candidate merge thresholds for entangled-area flow, descending. The first
/// entry is the MANDATORY floor: pairs with ≥30% cross-flow are always merged;
/// calibration may only merge *more* aggressively, never less.
const MERGE_TAUS: [f64; 6] = [0.30, 0.25, 0.20, 0.15, 0.12, 0.10];
/// Neighbour-count grid for the area vote.
const CAL_KS: [usize; 3] = [10, 15, 20];
/// Allowed own-area neighbours grid.
const CAL_ZS: [usize; 2] = [0, 1];
/// Hard cap on simulated in-place over-fire during calibration.
const CAL_OVERFIRE_CAP: f64 = 0.025;
/// Minimum simulated transplant recall for placement to be enabled at all.
const CAL_MIN_RECALL: f64 = 0.85;
/// Calibration samples at most this many functions (stride sampling) — bounds
/// the O(sample × index) neighbour scan on very large corpora.
const CAL_MAX_SAMPLE: usize = 8000;
/// Neighbours used for the entanglement flow matrix.
const FLOW_K: usize = 10;
/// Minimum neighbours required to vote at all (too few → no signal, abstain).
const MIN_NEIGHBORS: usize = 5;
/// Placement candidate substance floor: a stub's nearest neighbours are noise —
/// you cannot judge the architectural home of a 5-line delegator.
const MIN_PLACEMENT_BODY_LINES: usize = 6;

/// The directory a repo-relative path lives in (everything before the last `/`),
/// or `""` for a top-level file. Shared with the F1 scorer (same-directory
/// margin bar).
pub(super) fn parent_dir(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

/// The adaptive area walk: per-directory function counts from the indexed
/// paths, so any path (indexed or new candidate) maps to its base area.
pub struct AreaWalk {
    /// Function count per directory prefix ("" = repo root).
    counts: HashMap<String, usize>,
    /// Total functions (the root prefix count).
    total: usize,
}

impl AreaWalk {
    pub fn new<'a>(paths: impl Iterator<Item = &'a str>) -> Self {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for p in paths {
            let dirs: Vec<&str> = {
                let mut c: Vec<&str> = p.split('/').collect();
                c.pop(); // drop the filename
                c
            };
            let mut prefix = String::new();
            *counts.entry(prefix.clone()).or_insert(0) += 1;
            for d in dirs {
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(d);
                *counts.entry(prefix.clone()).or_insert(0) += 1;
            }
        }
        let total = counts.get("").copied().unwrap_or(0);
        Self { counts, total }
    }

    /// The base area of a repo-relative path: descend through containers,
    /// stop at the first non-dominant directory; tiny areas merge up into
    /// their container.
    pub fn area(&self, path: &str) -> String {
        let dirs: Vec<&str> = {
            let mut c: Vec<&str> = path.split('/').collect();
            c.pop();
            c
        };
        let mut prefix = String::new();
        for d in dirs {
            let next = if prefix.is_empty() {
                d.to_string()
            } else {
                format!("{prefix}/{d}")
            };
            let parent_n = self.counts.get(&prefix).copied().unwrap_or(0);
            let next_n = self.counts.get(&next).copied().unwrap_or(0);
            if (next_n as f64) > MAX_CONTAINER_FRAC * parent_n as f64
                || (next_n as f64) > ABS_CONTAINER_FRAC * self.total as f64
            {
                prefix = next; // container: descend
                continue;
            }
            if next_n >= MIN_AREA_FNS {
                return next;
            }
            return prefix; // tiny: merge up into the container
        }
        prefix // file directly in a container dir
    }
}

/// The per-repo, per-language placement configuration produced by fit-time
/// self-calibration and stored in the semantic artifact.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlacementConfig {
    /// False when no calibrated config reached usable simulated recall — the
    /// placement sense abstains entirely on this repo.
    pub enabled: bool,
    /// Neighbours polled for the area vote.
    pub k: usize,
    /// Own-area neighbours tolerated before "misplaced".
    pub z: usize,
    /// Base area → merged area label (the merged label is the largest member).
    pub area_map: BTreeMap<String, String>,
    /// Diagnostics: the simulated transplant recall / in-place over-fire the
    /// calibration selected this config at.
    pub sim_recall: f32,
    pub sim_overfire: f32,
}

/// A fired placement finding: where the function looks like it belongs.
///
/// Both areas are **base** areas — real directories the reader can open. The
/// vote runs over *merged* groups (entangled areas are not judgeable apart),
/// but a merged group is labelled after its biggest member, so rendering the
/// label names a directory the function is not in and the peers are not in.
/// Reporting the base areas instead keeps the sentence consistent with the
/// peer lines printed under it.
#[derive(Debug, Clone)]
pub struct MisplacedFinding {
    /// The directory the function is actually filed under.
    pub actual_area: String,
    /// Where the nearest neighbours live: the most common base area inside the
    /// modal merged group.
    pub neighbor_area: String,
    pub in_area_fraction: f32,
    /// The modal area's share of the vote — what "belonging" looks like here.
    pub expected_fraction: f32,
    /// Nearest peers (symbol, path, line) for evidence.
    pub peers: Vec<(String, String, usize)>,
}

/// Scores diff-defined functions for architectural misplacement.
pub struct PlacementScorer<'a> {
    index: &'a SemanticIndex,
    cfg: &'a PlacementConfig,
    walk: AreaWalk,
    /// Merged area per index entry (aligned with `index.entries`).
    entry_areas: Vec<String>,
    /// Every directory that held an indexed function at fit time. Placement can
    /// only judge a location with precedent.
    index_dirs: HashSet<String>,
}

impl<'a> PlacementScorer<'a> {
    pub fn new(index: &'a SemanticIndex, cfg: &'a PlacementConfig) -> Self {
        let walk = AreaWalk::new(index.entries.iter().map(|e| e.path.as_str()));
        let entry_areas = index
            .entries
            .iter()
            .map(|e| {
                let base = walk.area(&e.path);
                cfg.area_map.get(&base).cloned().unwrap_or(base)
            })
            .collect();
        let index_dirs = index
            .entries
            .iter()
            .map(|e| parent_dir(&e.path).to_string())
            .collect();
        Self {
            index,
            cfg,
            walk,
            entry_areas,
            index_dirs,
        }
    }

    /// Evaluate one diff-defined function; `Some` when it looks misplaced.
    pub fn evaluate(&self, func: &FunctionRef, query: &[f32]) -> Option<MisplacedFinding> {
        if !self.cfg.enabled || super::redundant::is_test_path(&func.path) {
            return None;
        }
        // Substance floor: a stub's neighbours are noise, not placement evidence.
        if func.text.lines().count() < MIN_PLACEMENT_BODY_LINES {
            return None;
        }
        // A function in a directory that did NOT exist at fit time is *new*, not
        // *misplaced* — placement can only judge a location the repo already has
        // an opinion about. (This is the dominant clean-commit F2 FP.)
        if !self.index_dirs.contains(parent_dir(&func.path)) {
            return None;
        }
        let base = self.walk.area(&func.path);
        let claimed = self.cfg.area_map.get(&base).cloned()?;
        // Exclude only the function itself (a new diff function isn't in the
        // index, so this is a no-op at check) — same-file siblings are
        // legitimate area evidence.
        let neigh = self.index.nearest(query, self.cfg.k, |e| {
            !(e.path == func.path && e.line == func.line)
        });
        if neigh.len() < MIN_NEIGHBORS {
            return None;
        }
        let areas: Vec<&str> = neigh
            .iter()
            .map(|n| self.entry_areas[n.entry_index].as_str())
            .collect();
        let counts = area_counts(&areas);
        let (modal, modal_n) = (&counts[0].0, counts[0].1);
        let own = areas.iter().filter(|a| **a == claimed).count();
        if *modal == claimed || own > self.cfg.z {
            return None;
        }
        // Report the base directory the neighbours in the modal group actually
        // live in, not the group's label — see [`MisplacedFinding`].
        let modal_base: Vec<String> = neigh
            .iter()
            .zip(&areas)
            .filter(|(_, a)| **a == *modal)
            .map(|(n, _)| self.walk.area(&self.index.entry(n.entry_index).path))
            .collect();
        let refs: Vec<&str> = modal_base.iter().map(String::as_str).collect();
        let neighbor_area = area_counts(&refs)
            .first()
            .map(|(a, _)| a.clone())
            .unwrap_or_else(|| modal.clone());
        let peers = neigh
            .iter()
            .take(3)
            .map(|n| {
                let e = self.index.entry(n.entry_index);
                (e.symbol.clone(), e.path.clone(), e.line)
            })
            .collect();
        Some(MisplacedFinding {
            actual_area: base,
            neighbor_area,
            in_area_fraction: own as f32 / areas.len() as f32,
            expected_fraction: modal_n as f32 / areas.len() as f32,
            peers,
        })
    }
}

/// Area → neighbour count, sorted by descending count then area name (so the
/// modal area is deterministic on ties).
fn area_counts(areas: &[&str]) -> Vec<(String, usize)> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for a in areas {
        *counts.entry(a).or_insert(0) += 1;
    }
    let mut v: Vec<(String, usize)> = counts
        .into_iter()
        .map(|(a, c)| (a.to_string(), c))
        .collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}

// --- fit-time self-calibration --------------------------------------------

/// Union-find over area ids.
struct Dsu(Vec<usize>);
impl Dsu {
    fn new(n: usize) -> Self {
        Dsu((0..n).collect())
    }
    fn find(&mut self, mut x: usize) -> usize {
        while self.0[x] != x {
            self.0[x] = self.0[self.0[x]];
            x = self.0[x];
        }
        x
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.0[ra] = rb;
        }
    }
}

/// Self-calibrate the placement sense on a freshly built index. See the module
/// docs for the design; mirrors the all-gates offline validation exactly.
pub fn calibrate_placement(index: &SemanticIndex) -> PlacementConfig {
    let n = index.len();
    let disabled = PlacementConfig::default();
    if n < MIN_AREA_FNS * 2 {
        return disabled;
    }
    let walk = AreaWalk::new(index.entries.iter().map(|e| e.path.as_str()));
    let base_areas: Vec<String> = index.entries.iter().map(|e| walk.area(&e.path)).collect();
    let mut uniq: Vec<String> = {
        let s: HashSet<&String> = base_areas.iter().collect();
        s.into_iter().cloned().collect()
    };
    uniq.sort();
    if uniq.len() < 2 {
        return disabled;
    }
    let area_id: HashMap<&str, usize> = uniq
        .iter()
        .enumerate()
        .map(|(i, a)| (a.as_str(), i))
        .collect();
    let fn_area: Vec<usize> = base_areas.iter().map(|a| area_id[a.as_str()]).collect();
    let mut area_fns = vec![0usize; uniq.len()];
    for &a in &fn_area {
        area_fns[a] += 1;
    }

    // Stride-sampled neighbour cache: top max(CAL_KS) per sampled function,
    // excluding only the function itself. Each query is an independent scan
    // of the (read-only) index — computed in parallel, results in sample
    // order, so the cache is identical to the sequential build (this loop was
    // ~85 s single-threaded on a 25k-function corpus).
    let step = n.div_ceil(CAL_MAX_SAMPLE).max(1);
    let kmax = *CAL_KS.iter().max().unwrap();
    let sample: Vec<usize> = (0..n).step_by(step).collect();
    let neigh: Vec<Vec<usize>> = argot_engine::par::par_map_indexed(sample.len(), |si| {
        let e = index.entry(sample[si]);
        let ns = index.nearest(&e.vec, kmax, |o| !(o.path == e.path && o.line == e.line));
        ns.into_iter().map(|x| x.entry_index).collect()
    });

    // Entanglement flow between base areas, from the sampled top-FLOW_K votes.
    let m = uniq.len();
    let mut flow = vec![vec![0f64; m]; m];
    let mut outdeg = vec![0f64; m];
    for (si, &qi) in sample.iter().enumerate() {
        let a = fn_area[qi];
        for &j in neigh[si].iter().take(FLOW_K) {
            flow[a][fn_area[j]] += 1.0;
            outdeg[a] += 1.0;
        }
    }
    for (row, &deg) in flow.iter_mut().zip(&outdeg) {
        if deg > 0.0 {
            for v in row.iter_mut() {
                *v /= deg;
            }
        }
    }

    // Grid search: merge threshold × (k, z), best simulated recall under the
    // over-fire cap.
    let mut best: Option<(PlacementConfig, f64)> = None;
    for &tau in &MERGE_TAUS {
        // Merge every pair with cross-flow ≥ tau (mandatory floor included,
        // since MERGE_TAUS starts at the floor).
        let mut dsu = Dsu::new(m);
        for (a, row) in flow.iter().enumerate() {
            for (b, &ab) in row.iter().enumerate().skip(a + 1) {
                if ab >= tau || flow[b][a] >= tau {
                    dsu.union(a, b);
                }
            }
        }
        // Merged label per group = biggest member area.
        let mut group_best: HashMap<usize, usize> = HashMap::new();
        for a in 0..m {
            let g = dsu.find(a);
            let cur = group_best.entry(g).or_insert(a);
            if area_fns[a] > area_fns[*cur] {
                *cur = a;
            }
        }
        let merged_of: Vec<usize> = (0..m).map(|a| group_best[&dsu.find(a)]).collect();
        let merged_areas: HashSet<usize> = merged_of.iter().copied().collect();
        if merged_areas.len() < 2 {
            continue;
        }
        let merged_list: Vec<usize> = {
            let mut v: Vec<usize> = merged_areas.into_iter().collect();
            v.sort();
            v
        };
        for &k in &CAL_KS {
            for &z in &CAL_ZS {
                let mut rec = 0usize;
                let mut rec_ev = 0usize;
                let mut of = 0usize;
                let mut of_ev = 0usize;
                for (si, &qi) in sample.iter().enumerate() {
                    let votes: Vec<usize> = neigh[si]
                        .iter()
                        .take(k)
                        .map(|&j| merged_of[fn_area[j]])
                        .collect();
                    if votes.len() < MIN_NEIGHBORS {
                        continue;
                    }
                    // Modal merged area (deterministic on ties: lowest id wins
                    // among equal counts — ids are sorted labels).
                    let mut counts: HashMap<usize, usize> = HashMap::new();
                    for &v in &votes {
                        *counts.entry(v).or_insert(0) += 1;
                    }
                    let modal = *counts
                        .iter()
                        .max_by(|x, y| x.1.cmp(y.1).then_with(|| y.0.cmp(x.0)))
                        .unwrap()
                        .0;
                    let actual = merged_of[fn_area[qi]];
                    let fires = |claimed: usize| {
                        modal != claimed && votes.iter().filter(|&&v| v == claimed).count() <= z
                    };
                    of_ev += 1;
                    if fires(actual) {
                        of += 1;
                    }
                    for &foreign in &merged_list {
                        if foreign == actual {
                            continue;
                        }
                        rec_ev += 1;
                        if fires(foreign) {
                            rec += 1;
                        }
                    }
                }
                if rec_ev == 0 || of_ev == 0 {
                    continue;
                }
                let r = rec as f64 / rec_ev as f64;
                let o = of as f64 / of_ev as f64;
                if o <= CAL_OVERFIRE_CAP && best.as_ref().map(|(_, br)| r > *br).unwrap_or(true) {
                    let area_map: BTreeMap<String, String> = (0..m)
                        .map(|a| (uniq[a].clone(), uniq[merged_of[a]].clone()))
                        .collect();
                    best = Some((
                        PlacementConfig {
                            enabled: true,
                            k,
                            z,
                            area_map,
                            sim_recall: r as f32,
                            sim_overfire: o as f32,
                        },
                        r,
                    ));
                }
            }
        }
    }
    match best {
        Some((cfg, r)) if r >= CAL_MIN_RECALL => cfg,
        _ => disabled,
    }
}

#[cfg(test)]
mod tests;
