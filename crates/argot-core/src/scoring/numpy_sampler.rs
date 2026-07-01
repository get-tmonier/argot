//! Bit-exact reproduction of numpy's `default_rng(seed).choice(pop, n, replace=False)`
//! for calibration hunk sampling.
//!
//! The Python engine samples calibration hunks with
//! `sorted(np.random.default_rng(seed).choice(len(candidates), n, replace=False))`
//! (see `engine/argot/scoring/calibration/random_hunk_sampler.py`). The BPE
//! threshold is `max(cal_scores)` over that sampled pool, so a *different* pool
//! yields a *different* threshold. To keep the calibrated threshold identical to
//! the Python engine on every corpus, this module reproduces numpy's RNG stream
//! and its `choice` algorithm exactly:
//!
//! 1. `SeedSequence(seed).generate_state(4, uint64)` → PCG64 seed + increment
//!    (`bit_generator.pyx`).
//! 2. PCG64 XSL-RR 128/64 output (`pcg64.h`).
//! 3. `choice(replace=False, p=None)` = Floyd's algorithm for small samples, or a
//!    tail Fisher-Yates shuffle when `pop > 10000 && n > pop/50` (`_generator.pyx`).
//!
//! Only the *set* of indices matters downstream (the caller sorts), so the
//! post-Floyd `_shuffle_int` is a no-op here and is omitted; the tail-shuffle
//! branch, whose shuffle *determines* the set, is reproduced in full.
//!
//! Verified against numpy 2.4.4 reference vectors (see tests).

const MASK32: u64 = 0xFFFF_FFFF;
const INIT_A: u32 = 0x43b0_d7e5;
const MULT_A: u32 = 0x931e_8875;
const INIT_B: u32 = 0x8b51_f9dd;
const MULT_B: u32 = 0x58f3_8ded;
const MIX_MULT_L: u32 = 0xca01_f9dd;
const MIX_MULT_R: u32 = 0x4973_f715;
const XSHIFT: u32 = 16;
const PCG_MULT: u128 = 0x2360_ed05_1fc6_5da4_4385_df64_9fcc_f645;

/// numpy `hashmix`: mixes `value` into the running `hash_const`.
fn hashmix(value: u32, hash_const: &mut u32) -> u32 {
    let mut v = value ^ *hash_const;
    *hash_const = hash_const.wrapping_mul(MULT_A);
    v = v.wrapping_mul(*hash_const);
    v ^= v >> XSHIFT;
    v
}

/// numpy `mix`: combines two mixed words.
fn mix(x: u32, y: u32) -> u32 {
    let mut r = MIX_MULT_L
        .wrapping_mul(x)
        .wrapping_sub(MIX_MULT_R.wrapping_mul(y));
    r ^= r >> XSHIFT;
    r
}

/// `_int_to_uint32_array(seed)` — little-endian u32 words; `[0]` for seed 0.
fn int_to_u32_array(seed: u64) -> Vec<u32> {
    if seed == 0 {
        return vec![0];
    }
    let mut n = seed;
    let mut out = Vec::new();
    while n > 0 {
        out.push((n & MASK32) as u32);
        n >>= 32;
    }
    out
}

/// `SeedSequence(seed).generate_state(4, uint64)` → 4 little-endian u64 words.
fn generate_state(seed: u64) -> [u64; 4] {
    let entropy = int_to_u32_array(seed);

    // mix_entropy → pool (size 4).
    let mut hash_const: u32 = INIT_A;
    let mut mixer = [0u32; 4];
    for (i, m) in mixer.iter_mut().enumerate() {
        let e = if i < entropy.len() { entropy[i] } else { 0 };
        *m = hashmix(e, &mut hash_const);
    }
    for i_src in 0..4 {
        for i_dst in 0..4 {
            if i_src != i_dst {
                mixer[i_dst] = mix(mixer[i_dst], hashmix(mixer[i_src], &mut hash_const));
            }
        }
    }
    for &e in entropy.iter().skip(4) {
        for m in mixer.iter_mut() {
            *m = mix(*m, hashmix(e, &mut hash_const));
        }
    }

    // generate_state(4, uint64): 8 u32 words, then reinterpret as 4 LE u64.
    let mut hc: u32 = INIT_B;
    let mut st = [0u32; 8];
    for (i, s) in st.iter_mut().enumerate() {
        let mut dv = mixer[i % 4];
        dv ^= hc;
        hc = hc.wrapping_mul(MULT_B);
        dv = dv.wrapping_mul(hc);
        dv ^= dv >> XSHIFT;
        *s = dv;
    }
    [
        st[0] as u64 | ((st[1] as u64) << 32),
        st[2] as u64 | ((st[3] as u64) << 32),
        st[4] as u64 | ((st[5] as u64) << 32),
        st[6] as u64 | ((st[7] as u64) << 32),
    ]
}

/// PCG64 (XSL-RR 128/64) with numpy's SeedSequence seeding + a buffered
/// `next_uint32` reader (low half first, then high half), matching
/// `random_bounded_uint64`'s 32-bit path.
struct Pcg64 {
    state: u128,
    inc: u128,
    has_u32: bool,
    buf_u32: u32,
}

impl Pcg64 {
    fn new(seed: u64) -> Self {
        let v = generate_state(seed);
        let init_state = ((v[0] as u128) << 64) | (v[1] as u128);
        let init_seq = ((v[2] as u128) << 64) | (v[3] as u128);
        let mut rng = Pcg64 {
            state: 0,
            inc: (init_seq << 1) | 1,
            has_u32: false,
            buf_u32: 0,
        };
        rng.step();
        rng.state = rng.state.wrapping_add(init_state);
        rng.step();
        rng
    }

    fn step(&mut self) {
        self.state = self.state.wrapping_mul(PCG_MULT).wrapping_add(self.inc);
    }

    fn next_u64(&mut self) -> u64 {
        self.step();
        let s = self.state;
        let val = ((s >> 64) as u64) ^ (s as u64);
        let rot = (s >> 122) as u32;
        val.rotate_right(rot)
    }

    /// numpy `next_uint32`: returns the low 32 bits of a fresh u64, buffering the
    /// high 32 bits for the following call.
    fn next_u32(&mut self) -> u32 {
        if self.has_u32 {
            self.has_u32 = false;
            return self.buf_u32;
        }
        let n = self.next_u64();
        self.has_u32 = true;
        self.buf_u32 = (n >> 32) as u32;
        n as u32
    }

    /// `random_bounded_uint64(0, rng, 0, 0)` for `rng < 2^32` — Lemire's method
    /// over the buffered 32-bit stream. Returns a value in `[0, rng]`.
    fn bounded(&mut self, rng: u64) -> u64 {
        debug_assert!(rng < (1u64 << 32));
        if rng == 0 {
            return 0;
        }
        let rng_excl = rng + 1;
        let mut m = (self.next_u32() as u64).wrapping_mul(rng_excl);
        let mut leftover = m & MASK32;
        if leftover < rng_excl {
            let threshold = ((1u64 << 32).wrapping_sub(rng_excl)) % rng_excl;
            while leftover < threshold {
                m = (self.next_u32() as u64).wrapping_mul(rng_excl);
                leftover = m & MASK32;
            }
        }
        m >> 32
    }
}

/// `_shuffle_int(n, first, data)`: Fisher-Yates over `data[first..n]`, descending.
fn shuffle_int(rng: &mut Pcg64, data: &mut [usize], first: usize) {
    let n = data.len();
    let mut i = n;
    while i > first {
        i -= 1;
        let j = rng.bounded(i as u64) as usize;
        data.swap(i, j);
    }
}

/// `sorted(np.random.default_rng(seed).choice(pop, n, replace=False))`.
///
/// Returns `n` distinct indices from `[0, pop)`, sorted ascending. Caps `n` at
/// `pop` (matching the caller's `effective_n_cal = min(n_cal, len(candidates))`).
pub fn choice_sorted(pop: usize, n: usize, seed: u64) -> Vec<usize> {
    let n = n.min(pop);
    if n == 0 {
        return Vec::new();
    }
    let mut rng = Pcg64::new(seed);

    // choice(replace=False) default shuffle=True → cutoff 50.
    let cutoff = 50;
    if pop > 10000 && n > pop / cutoff {
        // Tail-shuffle branch: shuffle idx[max(pop-n,1)..pop], take the tail.
        let mut idx: Vec<usize> = (0..pop).collect();
        let first = (pop - n).max(1);
        shuffle_int(&mut rng, &mut idx, first);
        let mut tail: Vec<usize> = idx[pop - n..].to_vec();
        tail.sort_unstable();
        tail
    } else {
        // Floyd's algorithm. Only the set matters (caller sorts), so the
        // post-Floyd `_shuffle_int` is skipped.
        let mut chosen: Vec<usize> = Vec::with_capacity(n);
        let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::with_capacity(n);
        for j in (pop - n)..pop {
            let val = rng.bounded(j as u64);
            if seen.insert(val) {
                chosen.push(val as usize);
            } else {
                chosen.push(j);
                seen.insert(j as u64);
            }
        }
        chosen.sort_unstable();
        chosen
    }
}

#[cfg(test)]
mod tests {
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
}
