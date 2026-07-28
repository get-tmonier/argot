# The score grows with hunk size, so the threshold has to

**Date:** 2026-07-28 · **Status:** in progress — probes green on 10 corpora,
full run pending.

**Question:** whole-file rewrites flooded the benchmark's false alarms — 29,3 %
of every one came from hunks over 50 lines. A flat cap on hunk size fixed it,
badly: an arbitrary constant that had to be moved once (100 → 150) the moment it
ate a real fixture. What is actually wrong?

## The mechanism

`BpeScorer::bpe_score` is `max_surprise_over(...)` — a **max over the hunk's
tokens** of `ln(generic_freq) − ln(repo_freq)`. A max over N draws grows with N.
So a large hunk scores higher **for free**, with no change in how foreign it is,
and against a single scalar threshold it is mechanically closer to firing.

Extreme-value theory says the growth is logarithmic for any exponential-ish
tail. Measured over argot's own 2 672 calibration candidates, it is exactly
that:

| hunk lines | n | mean score |
|---|--:|--:|
| 6–10 | 748 | 1,68 |
| 11–20 | 987 | 1,79 |
| 21–40 | 574 | 2,14 |
| 41–80 | 241 | 2,42 |
| 81–160 | 90 | 2,94 |
| 161–320 | 23 | 3,29 |
| 321+ | 9 | **3,90** |

Fit: `0,515 + 0,551·ln(lines)` on the full score. Subtracting `β·ln(N)` collapses
the spread from 2,3× to ~1,5×.

## The fix

The threshold scales with size — `θ + β·ln(lines / reference)` — where **β and
the reference are fitted per language at calibration from the repo's own
sample**. No constant anywhere. Large hunks are *judged*, not skipped: a rewrite
can still fire, it just needs evidence proportional to its size.

Foreign imports bypass it entirely: an import is a membership test, not a max,
so the highest-precision signal cannot regress.

Every refusal to fit — too small a sample, too little size spread, a negative
slope — yields a zero slope, which is exactly today's flat threshold. Configs
fitted before the field exists read as zero, so the change is inert until refit.

## Two design errors, both caught by cheap probes

**1 · Double-counting the size effect. Cost: 34 catches (613/756).** The
threshold is calibrated as `max(cal_scores)`, and that max is typically attained
on a *large* candidate — so θ already carried a size bonus. Subtracting the
correction again at check charged every above-reference hunk twice. **Calibrate
in the space check compares in:** fit the slope first, then take the max over
*corrected* scores.

**2 · Calibrating over corrected scores, once the correction is clamped.**
Correcting inside calibration fixes the double-count only while the adjustment
is symmetric. Clamped below the reference — which is what keeps ordinary changes
judged as they are today — it *lowers* the threshold without lowering the bar for
the hunks that set it, so everything small fires more. The threshold therefore
stays `max(score)`, exactly as today, and the adjustment applies only above the
reference. Below it nothing changes at all, by construction.

**3 · Anchoring at the median. Cost: 3 catches on fmt.** Taxing everything above
the median penalises ordinary-but-largish changes. Binning the fit to make it
robust made fmt *worse* (slope 1,02 → 1,38) — the useful signal, because it said
the steep slope was real and the model was wrong, not the estimator.

The artefact only needs neutralising **in the tail**. Anchoring the reference at
**p90 of candidate sizes, clamped below**, leaves nine hunks in ten judged
exactly as they are today and bends the bar only where rewrites live.

## The guarantee this shape gives

With the threshold left at `max(score)` and the adjustment clamped at ≥ 0, the
effective bar can only **rise**. The hunks that fire are therefore a subset of
those that fire without it: **false alarms cannot increase, ever.** Only recall
is at risk, which is exactly what the probes below measure — and it is why the
reference sits at p90 rather than the median.

A corollary worth remembering: an apparent FP *increase* under this change is
impossible, so it means the baseline being compared against came from different
code. That is what "fastapi 1,46 % → 1,75 %" turned out to be — an A/B under
identical code gives **30/1 718 either way**.

## Results

Recall, against each corpus's known baseline:

| corpus | baseline | size-aware |
|---|--:|--:|
| fastapi | 21/25 | 21/25 |
| fmt | 19/23 | 19/23 |
| hono | 23/30 | 23/30 |
| curl | 18/22 | 18/22 |
| castle-engine | 11/11 | 11/11 |
| mseide-msegui | 8/10 | 8/10 |
| excalidraw | 23/27 | 23/27 |
| rocksdb | 21/26 | 21/26 |
| hugo | 20/22 | 20/22 |
| mormot2 | 11/11 | 11/11 |

**Ten corpora across six languages, all unchanged.** False alarms on the corpus
the whole thing is about:

| uos | |
|---|--:|
| existing over-fire, baseline | 3,09 % (108/3 492) |
| under the flat cap, at best | ~2,4 % — **and it cost a catch** |
| size-aware | **0,00 %** (0/3 492) |

## Why this shape is right

The fitted slopes say it. **uos fits 2,731** — its history is full of whole-file
rewrites, so it learns a steep penalty for large hunks. **curl fits 0,629** — it
does not, so its bar barely bends. argot's own repo fits 0,594 (rust) and 0,536
(typescript), matching the 0,551 measured independently.

The penalty is learned from how each repository actually works, rather than
imposed by a number someone picked. That is the difference between this and the
cap, and it is why the cap had to be re-tuned the first time it met a fixture it
had not been fitted against.

## The lesson worth keeping

A constant that needs tuning against the fixture set is usually a proxy for a
model that should have handled the thing. The cap was measurable, testable, and
wrong: it abstained instead of judging, and its number encoded "how big is too
big" — a question the repository can answer for itself.
