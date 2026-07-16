//! Receiver-namespace coverage divergence.
//!
//! Jensen-Shannon distance (base-2, range [0, 1]) between the hunk's
//! namespace-prefix distribution and the cluster's pooled distribution.
//! Contribution = `min(clip, js_distance * clip)`.

use crate::scoring::adapters::Language;
use crate::scoring::call_receiver::extract_callees;
use crate::scoring::shape_primitive::{Baseline, ShapePrimitive};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

/// Fewer than this many files with a callee → too sparse; abstain on fit.
const MIN_FILES: usize = 3;

/// Namespace prefix: first segment before `.`, or the whole callee if bare.
fn namespace_prefix(callee: &str) -> &str {
    callee
        .split_once('.')
        .map(|(head, _)| head)
        .unwrap_or(callee)
}

/// Jensen-Shannon distance: `sqrt(JSD)` with base-2 logs, range [0, 1].
/// `JSD = (KL(P||M) + KL(Q||M)) / 2`, `M = (P + Q) / 2`. Zero-probability
/// entries are masked (0·log0 = 0). Minor overshoot is clamped to [0, 1].
fn jsd_distance(p: &[f64], q: &[f64]) -> f64 {
    let mut kl_pm = 0.0;
    let mut kl_qm = 0.0;
    for i in 0..p.len() {
        let m = (p[i] + q[i]) / 2.0;
        if p[i] > 0.0 {
            kl_pm += p[i] * (p[i] / m).log2();
        }
        if q[i] > 0.0 {
            kl_qm += q[i] * (q[i] / m).log2();
        }
    }
    let jsd = ((kl_pm + kl_qm) / 2.0).clamp(0.0, 1.0);
    jsd.sqrt()
}

/// Receiver-namespace coverage divergence primitive.
#[derive(Default)]
pub struct NamespaceJsd;

impl ShapePrimitive for NamespaceJsd {
    fn name(&self) -> &str {
        "namespace_jsd"
    }
    fn min_cluster_size(&self) -> usize {
        10
    }
    fn cluster_bonus_clip(&self) -> f64 {
        5.0
    }

    fn fit_cluster_baseline(
        &self,
        cluster_files: &[(PathBuf, String)],
        language: Language,
    ) -> Option<Baseline> {
        let mut namespace_counts: HashMap<String, u64> = HashMap::new();
        let mut files_with_callees = 0usize;

        for (_path, source) in cluster_files {
            let callees: Vec<String> = extract_callees(source, language)
                .into_iter()
                .flatten()
                .collect();
            if callees.is_empty() {
                continue;
            }
            files_with_callees += 1;
            for callee in &callees {
                let ns = namespace_prefix(callee).to_string();
                *namespace_counts.entry(ns).or_insert(0) += 1;
            }
        }

        if files_with_callees < MIN_FILES {
            return None;
        }
        if namespace_counts.len() < 2 {
            return None;
        }

        let total: u64 = namespace_counts.values().sum();
        let total = total as f64;
        let distribution: BTreeMap<String, f64> = namespace_counts
            .iter()
            .map(|(ns, &count)| (ns.clone(), count as f64 / total))
            .collect();
        let alphabet: BTreeSet<String> = namespace_counts.keys().cloned().collect();
        Some(Baseline::Namespace {
            language,
            alphabet,
            distribution,
        })
    }

    fn score(&self, hunk: &str, baseline: Option<&Baseline>, cluster_size: usize) -> f64 {
        let Some(Baseline::Namespace {
            language,
            alphabet,
            distribution,
        }) = baseline
        else {
            return 0.0;
        };
        if cluster_size < self.min_cluster_size() {
            return 0.0;
        }

        let callees: Vec<String> = extract_callees(hunk, *language)
            .into_iter()
            .flatten()
            .collect();
        if callees.is_empty() {
            return 0.0;
        }

        let mut hunk_counts: HashMap<String, u64> = HashMap::new();
        for callee in &callees {
            *hunk_counts
                .entry(namespace_prefix(callee).to_string())
                .or_insert(0) += 1;
        }

        // `alphabet` is a BTreeSet, so iteration is already sorted (matches
        // Python `sorted(baseline.alphabet)`). One slot per prefix + one OOV.
        let alphabet_list: Vec<&String> = alphabet.iter().collect();
        let n = alphabet_list.len() + 1;
        let oov_idx = alphabet_list.len();

        let hunk_total: f64 = hunk_counts.values().sum::<u64>() as f64;
        let mut cluster_vec = vec![0.0f64; n];
        let mut hunk_vec = vec![0.0f64; n];

        for (i, ns) in alphabet_list.iter().enumerate() {
            cluster_vec[i] = distribution.get(*ns).copied().unwrap_or(0.0);
            hunk_vec[i] = hunk_counts.get(*ns).copied().unwrap_or(0) as f64 / hunk_total;
        }
        for (ns, &count) in &hunk_counts {
            if !alphabet.contains(ns) {
                hunk_vec[oov_idx] += count as f64 / hunk_total;
            }
        }

        let js_distance = jsd_distance(&cluster_vec, &hunk_vec);
        (js_distance * self.cluster_bonus_clip()).min(self.cluster_bonus_clip())
    }
}
