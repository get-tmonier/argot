//! Cluster-staple deficit — a shape-primitive candidate.
//!
//! "Staples" are the cluster's top-10 most-attested callees (per-file
//! presence, first-insertion tie-break). Per source, the scalar is the
//! fraction of staples the source does NOT invoke; a hunk sitting far above
//! its cluster's mean deficit is missing the cluster-typical vocabulary —
//! the "absence is the anomaly" shape. One-sided tail-z ramp; abstains on
//! sources with < 2 call nodes.

use crate::scoring::adapters::Language;
use crate::scoring::call_receiver::extract_callees;
use crate::scoring::shape_primitive::{Baseline, ShapePrimitive};
use crate::scoring::shape_primitives::population_mean_std;
use std::cell::Cell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

const TOP_N: usize = 10;
const MIN_VALID_FILES: usize = 3;
const MIN_CALLS: usize = 2;

/// Fraction of `staples` absent from `source`'s callee set. `None` when the
/// source has fewer than [`MIN_CALLS`] call nodes or the staple set is empty.
fn staple_deficit(source: &str, language: Language, staples: &BTreeSet<String>) -> Option<f64> {
    if staples.is_empty() {
        return None;
    }
    let callees = extract_callees(source, language);
    if callees.len() < MIN_CALLS {
        return None;
    }
    let present: HashSet<&str> = callees.iter().filter_map(|c| c.as_deref()).collect();
    let missing = staples
        .iter()
        .filter(|s| !present.contains(s.as_str()))
        .count();
    Some(missing as f64 / staples.len() as f64)
}

/// Top-10 cluster callees by per-file presence with first-insertion tie-break
/// (matches `Counter.most_common` semantics used across the primitives).
fn cluster_staples(cluster_files: &[(PathBuf, String)], language: Language) -> BTreeSet<String> {
    let mut order: Vec<String> = Vec::new();
    let mut counts: HashMap<String, u64> = HashMap::new();
    for (_path, source) in cluster_files {
        let mut seen: HashSet<String> = HashSet::new();
        for callee in extract_callees(source, language).into_iter().flatten() {
            if seen.insert(callee.clone()) {
                match counts.get_mut(&callee) {
                    Some(v) => *v += 1,
                    None => {
                        counts.insert(callee.clone(), 1);
                        order.push(callee);
                    }
                }
            }
        }
    }
    let mut entries: Vec<(usize, &String, u64)> = order
        .iter()
        .enumerate()
        .map(|(i, c)| (i, c, counts[c]))
        .collect();
    entries.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    entries
        .iter()
        .take(TOP_N)
        .map(|(_, c, _)| (*c).clone())
        .collect()
}

/// Cluster-staple deficit primitive. Language captured on fit.
#[derive(Default)]
pub struct ClusterStapleDeficit {
    language: Cell<Option<Language>>,
}

impl ShapePrimitive for ClusterStapleDeficit {
    fn name(&self) -> &str {
        "cluster_staple_deficit"
    }
    fn min_cluster_size(&self) -> usize {
        10
    }
    fn cluster_bonus_clip(&self) -> f64 {
        5.0
    }

    fn set_language(&self, language: Language) {
        self.language.set(Some(language));
    }

    fn fit_cluster_baseline(
        &self,
        cluster_files: &[(PathBuf, String)],
        language: Language,
    ) -> Option<Baseline> {
        self.language.set(Some(language));
        let staples = cluster_staples(cluster_files, language);
        let mut deficits: Vec<f64> = Vec::new();
        for (_path, source) in cluster_files {
            if let Some(d) = staple_deficit(source, language, &staples) {
                deficits.push(d);
            }
        }
        if deficits.len() < MIN_VALID_FILES {
            return None;
        }
        let (mean, std) = population_mean_std(&deficits);
        Some(Baseline::Top10MeanStd {
            top10_set: staples,
            mean,
            std,
        })
    }

    fn score(&self, hunk: &str, baseline: Option<&Baseline>, cluster_size: usize) -> f64 {
        let Some(Baseline::Top10MeanStd {
            top10_set,
            mean,
            std,
        }) = baseline
        else {
            return 0.0;
        };
        if cluster_size < self.min_cluster_size() {
            return 0.0;
        }
        let Some(language) = self.language.get() else {
            return 0.0;
        };
        let Some(deficit) = staple_deficit(hunk, language, top10_set) else {
            return 0.0;
        };
        // One-sided: positive z = hunk misses more staples than typical.
        let z = (deficit - *mean) / std.max(1e-6);
        (z - 1.0).max(0.0).min(self.cluster_bonus_clip())
    }
}

#[cfg(test)]
mod tests;
