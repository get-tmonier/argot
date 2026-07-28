//! Typical-call-density under-coverage.
//!
//! Per-file density = (#call nodes whose non-`None` callee is in the cluster's
//! top-10) / (total call nodes, including unresolved callees). The top-10 is
//! ranked by per-file PRESENCE count with first-insertion tie-break (document
//! order across files), matching Python's `Counter.most_common(10)`.
//! One-sided tail-z ramp (fires only on under-coverage).

use crate::scoring::adapters::Language;
use crate::scoring::call_receiver::extract_callees;
use crate::scoring::shape_primitive::{Baseline, ShapePrimitive};
use crate::scoring::shape_primitives::population_mean_std;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::OnceLock;

const TOP_N: usize = 10;
const MIN_VALID_FILES: usize = 3;

/// Fraction of call nodes whose callee is in `top10_set`. `None` when the
/// source has 0 call nodes (undefined denominator). Unresolved (`None`)
/// callees count toward the denominator but never the numerator.
fn compute_density(source: &str, language: Language, top10_set: &BTreeSet<String>) -> Option<f64> {
    let callees = extract_callees(source, language);
    let denom = callees.len();
    if denom == 0 {
        return None;
    }
    let hits = callees
        .iter()
        .filter(|c| c.as_deref().map(|s| top10_set.contains(s)).unwrap_or(false))
        .count();
    Some(hits as f64 / denom as f64)
}

/// Typical-call-density under-coverage primitive. Language captured on fit.
#[derive(Default)]
pub struct TypicalCallDensity {
    language: OnceLock<Language>,
}

impl ShapePrimitive for TypicalCallDensity {
    fn name(&self) -> &str {
        "typical_call_density"
    }
    fn min_cluster_size(&self) -> usize {
        10
    }
    fn cluster_bonus_clip(&self) -> f64 {
        5.0
    }

    fn set_language(&self, language: Language) {
        let _ = self.language.set(language);
    }

    fn fit_cluster_baseline(
        &self,
        cluster_files: &[(PathBuf, String)],
        language: Language,
    ) -> Option<Baseline> {
        let _ = self.language.set(language);

        // Per-file presence counts, preserving first-insertion order so the
        // top-10 tie-break matches Counter.most_common.
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

        // most_common(TOP_N): stable sort by count desc, first-insertion for
        // ties (explicit index tiebreak), take TOP_N.
        let mut entries: Vec<(usize, &String, u64)> = order
            .iter()
            .enumerate()
            .map(|(i, c)| (i, c, counts[c]))
            .collect();
        entries.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
        let top10_set: BTreeSet<String> = entries
            .iter()
            .take(TOP_N)
            .map(|(_, c, _)| (*c).clone())
            .collect();

        let mut densities: Vec<f64> = Vec::new();
        for (_path, source) in cluster_files {
            if let Some(d) = compute_density(source, language, &top10_set) {
                densities.push(d);
            }
        }
        if densities.len() < MIN_VALID_FILES {
            return None;
        }
        let (mean, std) = population_mean_std(&densities);
        Some(Baseline::Top10MeanStd {
            top10_set,
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
        let Some(language) = self.language.get().copied() else {
            return 0.0;
        };
        if top10_set.is_empty() {
            return 0.0;
        }
        let Some(hunk_density) = compute_density(hunk, language, top10_set) else {
            return 0.0;
        };
        // One-sided: positive z = hunk below cluster mean = under-coverage.
        let z = (*mean - hunk_density) / std.max(1e-6);
        (z - 1.0).max(0.0).min(self.cluster_bonus_clip())
    }
}
