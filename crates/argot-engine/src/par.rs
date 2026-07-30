//! Deterministic data-parallelism for the feature-gated heavy phases
//! (semantic calibration/scoring, the integrity mini-replay).
//!
//! [`par_map_indexed`] is a drop-in for `(0..n).map(f).collect()`: contiguous
//! index chunks fan out over scoped threads and the per-index results are
//! reassembled **in input order**, so the output is element-for-element
//! identical to the sequential map — parallelism changes wall-clock only,
//! never a result, an aggregation order, or a downstream byte. `f` must be
//! pure per index (no cross-index state), which every call site here is.

use std::num::NonZeroUsize;

/// Global worker-thread cap from `ARGOT_THREADS` (≥ 1): the one operational
/// knob for sharing a machine — a pre-commit fit shouldn't saturate every
/// core. Honored by every heavy phase that runs through this pool. Unset or
/// invalid → no cap.
pub fn thread_cap() -> Option<usize> {
    parse_thread_cap(std::env::var("ARGOT_THREADS").ok().as_deref())
}

fn parse_thread_cap(raw: Option<&str>) -> Option<usize> {
    raw?.trim().parse::<usize>().ok().filter(|&n| n >= 1)
}

/// Map `f` over `0..n` on up to `available_parallelism` scoped threads
/// (capped by [`thread_cap`]), returning results in index order. Falls back
/// to a plain sequential map when `n` is small or only one core is available.
pub fn par_map_indexed<R: Send>(n: usize, f: impl Fn(usize) -> R + Sync) -> Vec<R> {
    let threads = std::thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(1)
        .min(thread_cap().unwrap_or(usize::MAX))
        .min(n);
    if threads <= 1 {
        return (0..n).map(f).collect();
    }
    let chunk = n.div_ceil(threads);
    let f = &f;
    let parts: Vec<Vec<R>> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let lo = t * chunk;
                let hi = ((t + 1) * chunk).min(n);
                s.spawn(move || (lo..hi).map(f).collect::<Vec<R>>())
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("par_map_indexed worker panicked"))
            .collect()
    });
    let mut out = Vec::with_capacity(n);
    for p in parts {
        out.extend(p);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_sequential_map_in_order() {
        for n in [0usize, 1, 2, 7, 100, 1023] {
            let par = par_map_indexed(n, |i| i * i + 1);
            let seq: Vec<usize> = (0..n).map(|i| i * i + 1).collect();
            assert_eq!(par, seq, "n={n}");
        }
    }

    #[test]
    fn thread_cap_parses_only_positive_integers() {
        assert_eq!(parse_thread_cap(Some("4")), Some(4));
        assert_eq!(parse_thread_cap(Some(" 2 ")), Some(2));
        assert_eq!(parse_thread_cap(Some("0")), None);
        assert_eq!(parse_thread_cap(Some("-1")), None);
        assert_eq!(parse_thread_cap(Some("all")), None);
        assert_eq!(parse_thread_cap(Some("")), None);
        assert_eq!(parse_thread_cap(None), None);
    }

    #[test]
    fn shared_read_only_state_is_visible_to_workers() {
        let base: Vec<usize> = (0..500).map(|i| i * 3).collect();
        let out = par_map_indexed(base.len(), |i| base[i] + 1);
        assert_eq!(out[499], 499 * 3 + 1);
        assert_eq!(out.len(), 500);
    }
}
