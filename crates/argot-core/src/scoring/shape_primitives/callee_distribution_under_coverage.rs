//! Callee-distribution under-coverage — era-14 phase C candidate (math
//! option ii).
//!
//! The cluster baseline is the pooled callee-occurrence distribution `p`.
//! Per source, the scalar is a one-sided KL-style divergence: only mass the
//! source UNDER-uses relative to the cluster counts, `Σ_c max(0, p(c) ·
//! ln(p(c)/q(c)))` with add-epsilon smoothing of the source distribution `q`.
//! A hunk far above the cluster's own mean divergence is failing to speak the
//! cluster's callee vocabulary. One-sided tail-z ramp; abstains on sources
//! with < 2 call nodes.

use crate::scoring::adapters::Language;
use crate::scoring::call_receiver::extract_callees;
use crate::scoring::shape_primitive::{Baseline, ShapePrimitive};
use crate::scoring::shape_primitives::population_mean_std;
use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

const MIN_VALID_FILES: usize = 3;
const MIN_CALLS: usize = 2;
/// Baseline support is capped to the most frequent callees so the payload
/// stays bounded on huge clusters.
const SUPPORT_CAP: usize = 50;
const SMOOTHING_EPS: f64 = 0.01;

fn under_coverage(
    source: &str,
    language: Language,
    distribution: &BTreeMap<String, f64>,
) -> Option<f64> {
    if distribution.is_empty() {
        return None;
    }
    let callees: Vec<String> = extract_callees(source, language)
        .into_iter()
        .flatten()
        .collect();
    if callees.len() < MIN_CALLS {
        return None;
    }
    let mut counts: HashMap<&str, f64> = HashMap::new();
    for c in &callees {
        *counts.entry(c.as_str()).or_insert(0.0) += 1.0;
    }
    let n = callees.len() as f64;
    let support = distribution.len() as f64;
    let denom = n + SMOOTHING_EPS * support;
    let mut divergence = 0.0;
    for (callee, &p) in distribution {
        let q = (counts.get(callee.as_str()).copied().unwrap_or(0.0) + SMOOTHING_EPS) / denom;
        if q < p {
            divergence += p * (p / q).ln();
        }
    }
    Some(divergence)
}

/// Callee-distribution under-coverage primitive. Language captured on fit.
#[derive(Default)]
pub struct CalleeDistributionUnderCoverage {
    language: Cell<Option<Language>>,
}

impl ShapePrimitive for CalleeDistributionUnderCoverage {
    fn name(&self) -> &str {
        "callee_distribution_under_coverage"
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

        // Pooled occurrence counts with first-insertion order for the
        // deterministic support cap.
        let mut order: Vec<String> = Vec::new();
        let mut counts: HashMap<String, u64> = HashMap::new();
        let mut total = 0u64;
        for (_path, source) in cluster_files {
            for callee in extract_callees(source, language).into_iter().flatten() {
                match counts.get_mut(&callee) {
                    Some(v) => *v += 1,
                    None => {
                        counts.insert(callee.clone(), 1);
                        order.push(callee);
                    }
                }
                total += 1;
            }
        }
        if total == 0 {
            return None;
        }
        let mut entries: Vec<(usize, &String, u64)> = order
            .iter()
            .enumerate()
            .map(|(i, c)| (i, c, counts[c]))
            .collect();
        entries.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
        let support: Vec<(&String, u64)> = entries
            .iter()
            .take(SUPPORT_CAP)
            .map(|(_, c, n)| (*c, *n))
            .collect();
        let support_total: u64 = support.iter().map(|(_, n)| n).sum();
        if support_total == 0 {
            return None;
        }
        let distribution: BTreeMap<String, f64> = support
            .into_iter()
            .map(|(c, n)| (c.clone(), n as f64 / support_total as f64))
            .collect();

        let mut divergences: Vec<f64> = Vec::new();
        for (_path, source) in cluster_files {
            if let Some(d) = under_coverage(source, language, &distribution) {
                divergences.push(d);
            }
        }
        if divergences.len() < MIN_VALID_FILES {
            return None;
        }
        let (mean, std) = population_mean_std(&divergences);
        Some(Baseline::DistributionMeanStd {
            distribution,
            mean,
            std,
        })
    }

    fn score(&self, hunk: &str, baseline: Option<&Baseline>, cluster_size: usize) -> f64 {
        let Some(Baseline::DistributionMeanStd {
            distribution,
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
        let Some(divergence) = under_coverage(hunk, language, distribution) else {
            return 0.0;
        };
        let z = (divergence - *mean) / std.max(1e-6);
        (z - 1.0).max(0.0).min(self.cluster_bonus_clip())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(sources: &[&str]) -> Vec<(PathBuf, String)> {
        sources
            .iter()
            .enumerate()
            .map(|(i, s)| (PathBuf::from(format!("f{i}.py")), s.to_string()))
            .collect()
    }

    #[test]
    fn abstains_below_min_calls() {
        let prim = CalleeDistributionUnderCoverage::default();
        let cluster = files(&["foo()\nbar()\nbaz()\n"; 5]);
        let baseline = prim
            .fit_cluster_baseline(&cluster, Language::Python)
            .unwrap();
        assert_eq!(prim.score("qux()\n", Some(&baseline), 10), 0.0);
    }

    #[test]
    fn abstains_below_cluster_size_floor() {
        let prim = CalleeDistributionUnderCoverage::default();
        let cluster = files(&["foo()\nbar()\nbaz()\n"; 5]);
        let baseline = prim
            .fit_cluster_baseline(&cluster, Language::Python)
            .unwrap();
        assert_eq!(prim.score("qux()\nquux()\n", Some(&baseline), 9), 0.0);
    }

    #[test]
    fn cluster_matching_hunk_contributes_zero() {
        let prim = CalleeDistributionUnderCoverage::default();
        let cluster = files(&["foo()\nbar()\nbaz()\n"; 6]);
        let baseline = prim
            .fit_cluster_baseline(&cluster, Language::Python)
            .unwrap();
        assert_eq!(
            prim.score("foo()\nbar()\nbaz()\n", Some(&baseline), 10),
            0.0
        );
    }

    #[test]
    fn language_agnostic_runs_on_typescript() {
        let prim = CalleeDistributionUnderCoverage::default();
        let cluster: Vec<(PathBuf, String)> = (0..5)
            .map(|i| {
                (
                    PathBuf::from(format!("f{i}.ts")),
                    "foo();\nbar();\nbaz();\n".to_string(),
                )
            })
            .collect();
        let baseline = prim
            .fit_cluster_baseline(&cluster, Language::Typescript)
            .unwrap();
        assert_eq!(
            prim.score("foo();\nbar();\nbaz();\n", Some(&baseline), 10),
            0.0
        );
    }
}
