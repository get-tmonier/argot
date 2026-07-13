use super::*;

/// Raw PCG64 stream must match `np.random.PCG64(seed).random_raw()`.
#[test]
fn pcg64_stream_matches_numpy() {
    // np.random.PCG64(0).random_raw(4)
    let expected: [u64; 4] = [
        11749869230777074271,
        4976686463289251617,
        755828109848996024,
        304881062738325533,
    ];
    let mut rng = Pcg64::new(0);
    for &e in &expected {
        assert_eq!(rng.next_u64(), e);
    }
}

fn check(pop: usize, n: usize, seed: u64, sum: usize, first3: &[usize], last3: &[usize]) {
    let idx = choice_sorted(pop, n, seed);
    assert_eq!(idx.len(), n.min(pop));
    assert_eq!(
        idx.iter().sum::<usize>(),
        sum,
        "sum mismatch pop={pop} seed={seed}"
    );
    assert_eq!(&idx[..3], first3, "head mismatch pop={pop} seed={seed}");
    assert_eq!(
        &idx[idx.len() - 3..],
        last3,
        "tail mismatch pop={pop} seed={seed}"
    );
    // distinct + in range
    let set: std::collections::HashSet<_> = idx.iter().collect();
    assert_eq!(set.len(), idx.len());
    assert!(idx.iter().all(|&i| i < pop));
}

/// Floyd-branch cases (n <= pop/50 or pop <= 10000), vs numpy 2.4.4.
#[test]
fn choice_floyd_matches_numpy() {
    check(120, 100, 0, 6396, &[0, 1, 2], &[117, 118, 119]);
    check(300, 100, 0, 14201, &[0, 1, 2], &[287, 288, 291]);
    check(777, 100, 2, 40974, &[28, 29, 38], &[763, 765, 774]);
    check(4999, 100, 0, 252039, &[13, 26, 40], &[4770, 4860, 4939]);
}

/// Tail-shuffle-branch cases (pop > 10000 && n > pop/50), vs numpy 2.4.4.
#[test]
fn choice_tail_shuffle_matches_numpy() {
    check(
        16678,
        500,
        0,
        4341860,
        &[45, 89, 104],
        &[16621, 16672, 16674],
    );
    check(
        20000,
        500,
        1,
        5096808,
        &[115, 140, 222],
        &[19964, 19967, 19982],
    );
}

#[test]
fn caps_at_pop() {
    // n >= pop → all indices, sorted.
    let idx = choice_sorted(50, 50, 0);
    assert_eq!(idx, (0..50).collect::<Vec<_>>());
    let idx = choice_sorted(10, 100, 7);
    assert_eq!(idx, (0..10).collect::<Vec<_>>());
}
