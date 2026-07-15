//! Statistics helpers — port of `engine/argot/stats.py` (plus the percentile
//! math reused by threshold calibration).
//!
//! `percentile` reproduces numpy's default linear interpolation; `auc`
//! reproduces `sklearn.metrics.roc_auc_score` for binary labels via the
//! average-rank (Mann–Whitney) identity, including tie handling.

/// numpy `np.percentile(sorted_vals, q)` with the default `"linear"`
/// interpolation. `sorted_vals` MUST be ascending and non-empty.
pub fn percentile(sorted_vals: &[f64], q: f64) -> f64 {
    let n = sorted_vals.len();
    debug_assert!(n > 0, "percentile of empty slice");
    if n == 1 {
        return sorted_vals[0];
    }
    let idx = (q / 100.0) * (n as f64 - 1.0);
    let lo = idx.floor() as usize;
    let hi = (lo + 1).min(n - 1);
    sorted_vals[lo] + (idx - lo as f64) * (sorted_vals[hi] - sorted_vals[lo])
}

/// The six-number summary `compute_percentiles` emits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Percentiles {
    pub min: f64,
    pub p25: f64,
    pub median: f64,
    pub p75: f64,
    pub p95: f64,
    pub max: f64,
}

/// Port of `compute_percentiles`.
pub fn compute_percentiles(scores: &[f64]) -> Percentiles {
    let mut sorted = scores.to_vec();
    // `total_cmp` gives a NaN-safe total order (a stray NaN sorts to an end
    // instead of panicking); for finite scores it is identical to `partial_cmp`.
    sorted.sort_by(|a, b| a.total_cmp(b));
    Percentiles {
        min: sorted[0],
        p25: percentile(&sorted, 25.0),
        median: percentile(&sorted, 50.0),
        p75: percentile(&sorted, 75.0),
        p95: percentile(&sorted, 95.0),
        max: sorted[n_minus_one(&sorted)],
    }
}

fn n_minus_one(v: &[f64]) -> usize {
    v.len() - 1
}

/// Port of `compute_auc`: ROC-AUC with `good_scores` labelled 0 (negative)
/// and `bad_scores` labelled 1 (positive). Matches
/// `sklearn.metrics.roc_auc_score` including ties (average ranks).
pub fn compute_auc(good_scores: &[f64], bad_scores: &[f64]) -> f64 {
    let n_neg = good_scores.len();
    let n_pos = bad_scores.len();
    // sklearn raises on a single-class input; the caller (compute_auc) always
    // provides both. Guard defensively.
    if n_pos == 0 || n_neg == 0 {
        return f64::NAN;
    }

    // (score, is_positive) over all samples, sorted ascending by score.
    let mut all: Vec<(f64, bool)> = Vec::with_capacity(n_pos + n_neg);
    all.extend(good_scores.iter().map(|&s| (s, false)));
    all.extend(bad_scores.iter().map(|&s| (s, true)));
    all.sort_by(|a, b| a.0.total_cmp(&b.0));

    // Average ranks (1-based); tied scores share the mean of their positions.
    let mut sum_pos_ranks = 0.0f64;
    let mut i = 0usize;
    while i < all.len() {
        let mut j = i + 1;
        while j < all.len() && all[j].0 == all[i].0 {
            j += 1;
        }
        // positions i..j (0-based) → ranks (i+1)..=j; average rank:
        let avg_rank = ((i + 1 + j) as f64) / 2.0;
        for item in &all[i..j] {
            if item.1 {
                sum_pos_ranks += avg_rank;
            }
        }
        i = j;
    }

    let n_pos_f = n_pos as f64;
    let n_neg_f = n_neg as f64;
    (sum_pos_ranks - n_pos_f * (n_pos_f + 1.0) / 2.0) / (n_pos_f * n_neg_f)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-12, "{a} != {b}");
    }

    #[test]
    fn percentiles_match_numpy() {
        // Golden from numpy on [0.1,0.4,0.35,0.8,0.2,0.2,0.9,0.05].
        let scores = [0.1, 0.4, 0.35, 0.8, 0.2, 0.2, 0.9, 0.05];
        let p = compute_percentiles(&scores);
        approx(p.min, 0.05);
        approx(p.p25, 0.175_000_000_000_000_02);
        approx(p.median, 0.275);
        approx(p.p75, 0.5);
        approx(p.p95, 0.865);
        approx(p.max, 0.9);
    }

    #[test]
    fn auc_perfect_separation() {
        // Golden from sklearn roc_auc_score = 1.0.
        let good = [0.1, 0.2, 0.2, 0.05];
        let bad = [0.4, 0.35, 0.8, 0.9];
        approx(compute_auc(&good, &bad), 1.0);
    }

    #[test]
    fn auc_with_cross_class_ties() {
        // Golden from sklearn = 0.7777777777777778.
        let good = [0.5, 0.5, 0.1];
        let bad = [0.5, 0.9, 0.5];
        approx(compute_auc(&good, &bad), 0.777_777_777_777_777_8);
    }

    #[test]
    fn percentiles_do_not_panic_on_nan() {
        // A degenerate NaN in the scores must not panic the sort (the old
        // `partial_cmp().unwrap()` did). `total_cmp` sorts it deterministically.
        let scores = [0.3, f64::NAN, 0.1, 0.9, 0.2];
        let p = compute_percentiles(&scores);
        // Finite endpoints are still recovered from the finite values.
        approx(p.min, 0.1);
    }

    #[test]
    fn auc_does_not_panic_on_nan() {
        // AUC over scores that include a NaN must not panic.
        let good = [0.1, f64::NAN, 0.2];
        let bad = [0.8, 0.9, 0.5];
        let _ = compute_auc(&good, &bad);
    }
}
