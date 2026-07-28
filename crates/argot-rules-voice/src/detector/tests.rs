use crate::scoring::calibration::{fit_size_slope, size_threshold_adjustment};

#[test]
fn nine_hunks_in_ten_are_judged_exactly_as_before() {
    // The reference is p90 of candidate sizes and the adjustment is clamped
    // below it, so ordinary changes see precisely today's threshold. Taxing
    // from the median up instead cost fmt three catches.
    for lines in [1, 5, 15, 40, 43, 44] {
        assert_eq!(
            size_threshold_adjustment(lines, 0.55, 44),
            0.0,
            "{lines} lines must be unaffected"
        );
    }
}

#[test]
fn only_the_tail_pays() {
    // bpe_score is a max over the hunk's tokens, so a bigger hunk scores higher
    // for free. uos' "Comment reordered for all the functions" is 2 564 lines
    // in one hunk and must clear a materially higher bar.
    // uos fits slope 2,731 with a p90 of 44 lines; its 2 564-line rewrite pays
    // heavily, while a 60-line change barely moves.
    let rewrite = size_threshold_adjustment(2564, 2.731, 44);
    assert!(rewrite > 10.0, "2564-line adjustment was {rewrite}");
    let ordinary = size_threshold_adjustment(60, 2.731, 44);
    assert!(ordinary < 1.0, "60-line adjustment was {ordinary}");
    // The bar only ever rises: below the reference there is ample calibration
    // data and the flat threshold is already right.
    assert!(size_threshold_adjustment(5, 2.731, 44) >= 0.0);
}

#[test]
fn no_fit_means_no_correction() {
    // Every refusal to fit must degrade to today's flat threshold exactly.
    assert_eq!(size_threshold_adjustment(5000, 0.0, 44), 0.0);
    assert_eq!(size_threshold_adjustment(5000, 0.55, 0), 0.0);
    assert_eq!(size_threshold_adjustment(0, 0.55, 44), 0.0);
}

#[test]
fn the_slope_is_recovered_from_a_sample() {
    // Synthesize score = 1.0 + 0.5*ln(lines) and check the fit finds it.
    let sized: Vec<(usize, f64)> = (1..=300)
        .map(|i| {
            let lines = 3 + (i % 200);
            (lines, 1.0 + 0.5 * (lines as f64).ln())
        })
        .collect();
    let (slope, reference) = fit_size_slope(&sized);
    assert!((slope - 0.5).abs() < 0.05, "slope was {slope}");
    assert!(reference > 0);
}

#[test]
fn a_sample_too_small_or_too_flat_refuses_to_fit() {
    // Refusing is always safe — it is exactly today's behaviour.
    let tiny: Vec<(usize, f64)> = (0..10).map(|i| (10 + i, 2.0)).collect();
    assert_eq!(fit_size_slope(&tiny).0, 0.0);
    // 100 points, all the same size: nothing to regress on.
    let flat: Vec<(usize, f64)> = (0..100).map(|i| (20, 1.0 + i as f64 * 0.01)).collect();
    assert_eq!(fit_size_slope(&flat).0, 0.0);
    // A negative fit means there is nothing to correct for.
    let falling: Vec<(usize, f64)> = (1..=200)
        .map(|i| {
            let lines = 3 + (i % 150);
            (lines, 10.0 - (lines as f64).ln())
        })
        .collect();
    assert_eq!(fit_size_slope(&falling).0, 0.0);
}
