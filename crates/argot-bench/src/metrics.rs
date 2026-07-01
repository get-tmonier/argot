//! Metric helpers — port of the retired `argot_bench.metrics`.
//!
//! `fp_rate` counts only *eligible* controls: hunks short-circuited before the
//! scorer ran (atypical, atypical_file, excluded_path, auto_generated) are not
//! true false positives.

/// Reasons excluded from FP-rate denominators and AUC control populations.
pub fn is_excluded_reason(reason: &str) -> bool {
    matches!(
        reason,
        "atypical" | "atypical_file" | "excluded_path" | "auto_generated"
    )
}

/// ROC-AUC with catalog breaks as positives and real-PR controls as negatives.
/// Rank-based (Mann–Whitney) with average ranks for ties — matches
/// `sklearn.metrics.roc_auc_score`.
pub fn auc(break_scores: &[f64], control_scores: &[f64]) -> f64 {
    if break_scores.is_empty() || control_scores.is_empty() {
        return 0.0;
    }
    let n_pos = break_scores.len();
    let mut all: Vec<(f64, bool)> = break_scores
        .iter()
        .map(|&s| (s, true))
        .chain(control_scores.iter().map(|&s| (s, false)))
        .collect();
    all.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Average ranks over tie groups (1-indexed ranks).
    let n = all.len();
    let mut rank_sum_pos = 0.0f64;
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && all[j + 1].0 == all[i].0 {
            j += 1;
        }
        let avg_rank = ((i + 1) + (j + 1)) as f64 / 2.0;
        for item in all.iter().take(j + 1).skip(i) {
            if item.1 {
                rank_sum_pos += avg_rank;
            }
        }
        i = j + 1;
    }
    let n_pos_f = n_pos as f64;
    let n_neg_f = control_scores.len() as f64;
    let u = rank_sum_pos - n_pos_f * (n_pos_f + 1.0) / 2.0;
    u / (n_pos_f * n_neg_f)
}

/// Coefficient of variation (population std / mean) over per-seed thresholds.
pub fn threshold_cv(thresholds: &[f64]) -> f64 {
    if thresholds.is_empty() {
        return 0.0;
    }
    let mean = thresholds.iter().sum::<f64>() / thresholds.len() as f64;
    if mean == 0.0 {
        return 0.0;
    }
    let var = thresholds.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / thresholds.len() as f64;
    var.sqrt() / mean
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auc_separable_is_one() {
        assert_eq!(auc(&[5.0, 6.0], &[1.0, 2.0, 3.0]), 1.0);
    }

    #[test]
    fn auc_random_is_half() {
        // Identical distributions → 0.5 via tie handling.
        assert!((auc(&[1.0, 2.0], &[1.0, 2.0]) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn auc_inverted_is_zero() {
        assert_eq!(auc(&[1.0], &[2.0, 3.0]), 0.0);
    }

    #[test]
    fn auc_empty_is_zero() {
        assert_eq!(auc(&[], &[1.0]), 0.0);
        assert_eq!(auc(&[1.0], &[]), 0.0);
    }

    #[test]
    fn cv_of_constant_is_zero() {
        assert_eq!(threshold_cv(&[4.0, 4.0, 4.0]), 0.0);
    }

    #[test]
    fn cv_matches_population_std() {
        // mean 3, population var = ((1-3)^2 + (5-3)^2 + 0)/3 = 8/3
        let cv = threshold_cv(&[1.0, 5.0, 3.0]);
        assert!((cv - (8.0f64 / 3.0).sqrt() / 3.0).abs() < 1e-12);
    }

    #[test]
    fn excluded_reasons() {
        assert!(is_excluded_reason("atypical"));
        assert!(is_excluded_reason("excluded_path"));
        assert!(!is_excluded_reason("none"));
        assert!(!is_excluded_reason("bpe"));
    }
}
