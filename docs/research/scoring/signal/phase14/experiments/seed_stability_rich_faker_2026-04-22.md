# Phase 14 Task #2 — Seed Stability: Rich and Faker

**Date:** 2026-04-22  
**Branch:** research/phase-14-import-graph  
**Why:** post-fix9, Rich uses N=230 and Faker uses N=250. Only FastAPI has been
directly probed for seed stability (N=500 confirmed stable). This probe verifies
Rich and Faker at their fix9 N values.

---

## §0 Headline

- **RICH** N=230: STABLE → recommended N=230 (stable)
- **FAKER** N=250: UNSTABLE → recommended N=350

---

## RICH corpus

**N=230 is STABLE** — both gates pass across all 5 seeds.
- Max threshold rel_var: 0.00% (gate: ≤10%)
- Min pairwise Jaccard: 100.00% (gate: ≥80%)

### §1 Per-seed Threshold Table

| PR# | seed0 | seed1 | seed2 | seed3 | seed4 | rel_var |
| --- | --- | --- | --- | --- | --- | --- |
| 3692 | 3.7622 | 3.7622 | 3.7622 | 3.7622 | 3.7622 | 0.00% |
| 3718 | 3.7620 | 3.7620 | 3.7620 | 3.7620 | 3.7620 | 0.00% |
| 3731 | 3.7617 | 3.7617 | 3.7617 | 3.7617 | 3.7617 | 0.00% |
| 3763 | 3.7627 | 3.7627 | 3.7627 | 3.7627 | 3.7627 | 0.00% |
| 3768 | 3.7617 | 3.7617 | 3.7617 | 3.7617 | 3.7617 | 0.00% |
| 3772 | 3.7611 | 3.7611 | 3.7611 | 3.7611 | 3.7611 | 0.00% |
| 3775 | 3.7617 | 3.7617 | 3.7617 | 3.7617 | 3.7617 | 0.00% |
| 3776 | 3.7618 | 3.7618 | 3.7618 | 3.7618 | 3.7618 | 0.00% |
| 3777 | 3.7618 | 3.7618 | 3.7618 | 3.7618 | 3.7618 | 0.00% |
| 3782 | 3.7622 | 3.7622 | 3.7622 | 3.7622 | 3.7622 | 0.00% |
| 3783 | 3.7619 | 3.7619 | 3.7619 | 3.7619 | 3.7619 | 0.00% |
| 3807 | 3.7622 | 3.7622 | 3.7622 | 3.7622 | 3.7622 | 0.00% |
| 3828 | 4.2014 | 4.2014 | 4.2014 | 4.2014 | 4.2014 | 0.00% |
| 3845 | 4.2026 | 4.2026 | 4.2026 | 4.2026 | 4.2026 | 0.00% |
| 3861 | 3.7623 | 3.7623 | 3.7623 | 3.7623 | 3.7623 | 0.00% |
| 3879 | 4.1997 | 4.1997 | 4.1997 | 4.1997 | 4.1997 | 0.00% |
| 3882 | 4.1997 | 4.1997 | 4.1997 | 4.1997 | 4.1997 | 0.00% |
| 3894 | 4.1995 | 4.1995 | 4.1995 | 4.1995 | 4.1995 | 0.00% |
| 3905 | 4.1994 | 4.1994 | 4.1994 | 4.1994 | 4.1994 | 0.00% |
| 3906 | 4.1978 | 4.1978 | 4.1978 | 4.1978 | 4.1978 | 0.00% |
| 3915 | 4.1978 | 4.1978 | 4.1978 | 4.1978 | 4.1978 | 0.00% |
| 3930 | 3.7623 | 3.7623 | 3.7623 | 3.7623 | 3.7623 | 0.00% |
| 3934 | 4.1975 | 4.1975 | 4.1975 | 4.1975 | 4.1975 | 0.00% |
| 3935 | 4.1995 | 4.1995 | 4.1995 | 4.1995 | 4.1995 | 0.00% |
| 3937 | 4.2000 | 4.2000 | 4.2000 | 4.2000 | 4.2000 | 0.00% |
| 3938 | 4.2001 | 4.2001 | 4.2001 | 4.2001 | 4.2001 | 0.00% |
| 3939 | 4.2010 | 4.2010 | 4.2010 | 4.2010 | 4.2010 | 0.00% |
| 3941 | 4.2029 | 4.2029 | 4.2029 | 4.2029 | 4.2029 | 0.00% |
| 3942 | 4.2010 | 4.2010 | 4.2010 | 4.2010 | 4.2010 | 0.00% |
| 3944 | 4.2014 | 4.2014 | 4.2014 | 4.2014 | 4.2014 | 0.00% |
| 3953 | 4.2014 | 4.2014 | 4.2014 | 4.2014 | 4.2014 | 0.00% |
| 4006 | 4.2014 | 4.2014 | 4.2014 | 4.2014 | 4.2014 | 0.00% |
| 4070 | 4.2025 | 4.2025 | 4.2025 | 4.2025 | 4.2025 | 0.00% |
| 4075 | 4.2031 | 4.2031 | 4.2031 | 4.2031 | 4.2031 | 0.00% |
| 4076 | 4.2032 | 4.2032 | 4.2032 | 4.2032 | 4.2032 | 0.00% |
| 4077 | 4.2032 | 4.2032 | 4.2032 | 4.2032 | 4.2032 | 0.00% |
| 4079 | 4.2033 | 4.2033 | 4.2033 | 4.2033 | 4.2033 | 0.00% |

### §2 Relative Variance Distribution

- PRs analysed: 37
- Max rel_var: 0.00%
- Median rel_var: 0.00%
- PRs with rel_var > 10%: 0
- PRs with rel_var > 5%: 0
- PRs with rel_var = 0%: 37

| rel_var range | count |
|---|---|
| 0%–1% | 37 |
| 1%–2% | 0 |
| 2%–5% | 0 |
| 5%–10% | 0 |
| 10%–20% | 0 |
| 20%–100% | 0 |

### §3 Flag Set Stability

- Total stable flag pairs (all 5 seeds agree): 23
- Total unstable flag pairs (some seeds disagree): 0
- Total unique flags across all seeds (union): 23
- Min pairwise Jaccard (seed-0 vs others): 100.00%

#### Pairwise Jaccard (seed-0 vs seed-k)

| seed | mean Jaccard | min Jaccard |
|---|---|---|
| seed-1 | 100.00% | 100.00% |
| seed-2 | 100.00% | 100.00% |
| seed-3 | 100.00% | 100.00% |
| seed-4 | 100.00% | 100.00% |

#### PRs with unstable flags

| PR# | stable | unstable | union | min_jaccard |
|---|---|---|---|---|
| — | — | — | — | — |

---

## FAKER corpus

**N=250 is UNSTABLE** — gate failure(s):
- Threshold gate FAIL: max rel_var = 13.96% > 10%
- Jaccard gate FAIL: min Jaccard = 0.00% < 80%
- Recommended N: **N=350**

### §5 Per-seed Threshold Table

| PR# | seed0 | seed1 | seed2 | seed3 | seed4 | rel_var |
| --- | --- | --- | --- | --- | --- | --- |
| 2211 | 5.2637 | 4.9398 | 5.2637 | 5.2637 | 5.2637 | 6.23% |
| 2214 | 5.2637 | 4.9398 | 5.2637 | 5.2637 | 5.2637 | 6.23% |
| 2220 | 5.2637 | 4.9398 | 5.2637 | 5.2637 | 5.2637 | 6.23% |
| 2225 | 5.2637 | 4.9398 | 5.2637 | 5.2637 | 5.2637 | 6.23% |
| 2230 | 5.2639 | 4.9400 | 5.2639 | 5.2639 | 5.2639 | 6.23% |
| 2232 | 5.2647 | 4.9407 | 5.2647 | 5.2647 | 5.2647 | 6.23% |
| 2243 | 5.2651 | 4.9412 | 5.2651 | 5.2651 | 5.2651 | 6.23% |
| 2246 | 5.2654 | 4.9415 | 5.2654 | 5.2654 | 5.2654 | 6.23% |
| 2251 | 5.2654 | 4.9415 | 5.2654 | 5.2654 | 5.2654 | 6.23% |
| 2255 | 5.2654 | 4.9415 | 5.2654 | 5.2654 | 5.2654 | 6.23% |
| 2256 | 5.2654 | 4.9415 | 5.2654 | 5.2654 | 5.2654 | 6.23% |
| 2259 | 5.2695 | 4.9455 | 5.2695 | 5.2695 | 5.2695 | 6.22% |
| 2263 | 5.3227 | 4.9979 | 5.3227 | 5.3227 | 5.3227 | 6.18% |
| 2264 | 5.3206 | 4.9958 | 5.3206 | 5.3206 | 5.3206 | 6.18% |
| 2265 | 5.3199 | 4.9951 | 5.3199 | 5.3199 | 5.3199 | 6.18% |
| 2267 | 5.3198 | 4.9950 | 5.3198 | 5.3198 | 5.3198 | 6.18% |
| 2270 | 5.3203 | 4.9956 | 5.3203 | 5.3203 | 5.3203 | 6.18% |
| 2271 | 5.3204 | 4.9956 | 5.3204 | 5.3204 | 5.3204 | 6.18% |
| 2272 | 5.3199 | 4.9951 | 5.3199 | 5.3199 | 5.3199 | 6.18% |
| 2275 | 5.3206 | 4.9958 | 5.3206 | 5.3206 | 5.3206 | 6.18% |
| 2276 | 5.3206 | 4.9958 | 5.3206 | 5.3206 | 5.3206 | 6.18% |
| 2279 | 5.3204 | 4.9956 | 5.3204 | 5.3204 | 5.3204 | 6.18% |
| 2287 | 5.3229 | 4.9981 | 5.3229 | 5.3229 | 5.3229 | 6.18% |
| 2291 | 5.3206 | 4.9958 | 5.3206 | 5.3206 | 5.3206 | 6.18% |
| 2294 | 5.3215 | 4.9967 | 5.3215 | 5.3215 | 5.3215 | 6.18% |
| 2299 | 5.3206 | 4.9958 | 5.3206 | 5.3206 | 5.3206 | 6.18% |
| 2302 | 5.3215 | 4.9967 | 5.3215 | 5.3215 | 5.3215 | 6.18% |
| 2304 | 5.3218 | 4.9969 | 5.3218 | 5.3218 | 5.3218 | 6.18% |
| 2306 | 5.3218 | 4.9969 | 5.3218 | 5.3218 | 5.3218 | 6.18% |
| 2309 | 5.3215 | 4.9967 | 5.3215 | 5.3215 | 5.3215 | 6.18% |
| 2310 | 5.3224 | 4.9976 | 5.3224 | 5.3224 | 5.3224 | 6.18% |
| 2314 | 5.3230 | 4.9981 | 5.3230 | 5.3230 | 5.3230 | 6.18% |
| 2316 | 5.3230 | 4.9981 | 5.3230 | 5.3230 | 5.3230 | 6.18% |
| 2318 | 5.3230 | 4.9981 | 5.3230 | 5.3230 | 5.3230 | 6.18% |
| 2324 | 5.3230 | 4.9981 | 5.3230 | 5.3230 | 5.3230 | 6.18% |
| 2326 | 5.3230 | 4.9981 | 5.3230 | 5.3230 | 5.3230 | 6.18% |
| 2327 | 5.3230 | 4.9981 | 5.3230 | 5.3230 | 5.3230 | 6.18% |
| 2330 | 5.3230 | 4.9981 | 5.3230 | 5.3230 | 5.3230 | 6.18% |
| 2337 | 5.3230 | 4.9981 | 5.3230 | 5.3230 | 5.3230 | 6.18% |
| 2340 | 5.3229 | 4.9980 | 5.3229 | 5.3229 | 5.3229 | 6.18% |
| 2341 | 5.3229 | 4.9981 | 5.3229 | 5.3229 | 5.3229 | 6.18% |
| 2347 | 5.3232 | 5.3232 | 5.3232 | 5.3232 | 4.6004 | 13.96% |
| 2348 | 5.3246 | 5.3246 | 5.3246 | 5.3246 | 4.6017 | 13.95% |
| 2349 | 5.3246 | 5.3246 | 5.3246 | 5.3246 | 4.6017 | 13.95% |
| 2350 | 5.3251 | 5.3251 | 5.3251 | 5.3251 | 4.6022 | 13.95% |
| 2351 | 5.3251 | 5.3251 | 5.3251 | 5.3251 | 4.6022 | 13.95% |
| 2352 | 5.3251 | 5.3251 | 5.3251 | 5.3251 | 4.6022 | 13.95% |
| 2358 | 5.3230 | 4.9981 | 5.3230 | 5.3230 | 5.3230 | 6.18% |
| 2362 | 5.3230 | 4.9981 | 5.3230 | 5.3230 | 5.3230 | 6.18% |
| 2364 | 5.3230 | 4.9981 | 5.3230 | 5.3230 | 5.3230 | 6.18% |

### §6 Relative Variance Distribution

- PRs analysed: 50
- Max rel_var: 13.96%
- Median rel_var: 6.18%
- PRs with rel_var > 10%: 6
- PRs with rel_var > 5%: 50
- PRs with rel_var = 0%: 0

| rel_var range | count |
|---|---|
| 0%–1% | 0 |
| 1%–2% | 0 |
| 2%–5% | 0 |
| 5%–10% | 44 |
| 10%–20% | 6 |
| 20%–100% | 0 |

### §7 Flag Set Stability

- Total stable flag pairs (all 5 seeds agree): 16
- Total unstable flag pairs (some seeds disagree): 4
- Total unique flags across all seeds (union): 20
- Min pairwise Jaccard (seed-0 vs others): 0.00%

#### Pairwise Jaccard (seed-0 vs seed-k)

| seed | mean Jaccard | min Jaccard |
|---|---|---|
| seed-1 | 96.00% | 0.00% |
| seed-2 | 100.00% | 100.00% |
| seed-3 | 100.00% | 100.00% |
| seed-4 | 98.00% | 0.00% |

#### PRs with unstable flags

| PR# | stable | unstable | union | min_jaccard |
|---|---|---|---|---|
| 2330 | 0 | 1 | 1 | 0.00% |
| 2341 | 0 | 1 | 1 | 0.00% |
| 2350 | 0 | 2 | 2 | 0.00% |

### §8 N-sweep

| N | max_rel_var | min_jaccard | thresh_gate | jaccard_gate | stable? |
|---|---|---|---|---|---|
| 250 | 13.96% | 0.00% | FAIL | FAIL | NO |
| 350 | 0.00% | 100.00% | pass | pass | YES |
| 500 | 0.00% | 100.00% | pass | pass | YES |

**Recommended N: 350**

---

## §9 Faker Borderline Flag Stability

Checking whether the 4 borderline flags from fix9 (bank/__init__.py, pyfloat,
address/en_GB, dataclass_migration) are stable across seeds at N=250.

### Borderline source flags

| file | hunk_idx | seed0_flagged | seed1_flagged | seed2_flagged | seed3_flagged | seed4_flagged | stable? |
|---|---|---|---|---|---|---|---|
| faker/providers/python/__init__.py | 0 | Y | Y | Y | Y | Y | YES |
| faker/providers/address/en_GB/__init__.py | 0 | Y | Y | Y | Y | Y | YES |
| faker/providers/bank/__init__.py | 0 | Y | Y | Y | Y | Y | YES |

**Good news:** all 3 borderline source flags are stable at N=250 — they flag in all 5 seeds.
The instability at N=250 hits different PRs (2330, 2341, 2350), not the borderline margin cases.

The dataclass_migration fixture (test hunk) was not found in the scored PR set — it may
fall outside the test-hunk scoring path or in a PR not in the base-rate set.

---

## §10 Implication

**FAKER**: N=250 is seed-fragile overall (3 PRs have unstable flag sets: 2330, 2341, 2350),
so **the fix9 faker base rate should be re-run at N=350** before treating the 18-flag count
as a settled reference point.

However, the specific borderline margin flags from fix9 (bank/__init__.py +0.09, pyfloat
+0.37, address/en_GB +0.41) are **all stable** across seeds at N=250 — they do not flip.
The seed-fragility risk for these flags is lower than expected.

The instability is in the threshold, not the flag identity for those files. At N=250, seed-1
draws a hunk that pushes the threshold ~6% lower (~4.94 vs 5.26), which flips marginal hunks
in PRs 2330, 2341, 2350. Raising to N=350 eliminates this sensitivity entirely.


