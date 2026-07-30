# Can the semantic index be committed to the repository?

**Date:** 2026-07-29
**Question:** the fitted semantic index is 58.6 MB and is rebuilt from scratch in
every environment — 25 min on a CI runner (see `semantic-seed-cost.md`). If it
were a *versioned file*, CI would never fit, every contributor would hold the
same model, and the cost would be paid once on a developer machine rather than
repeatedly on a runner. Is that reachable?
**Verdict:** on **size**, yes and comfortably — **5.2 MB committed, 0.25 MB per
refit**, with zero change to any `redundant` gate decision. On **byte-identity
across machines**, only by pinning the inference backend; and one gate
(x86 vs ARM) is still untested.

Corpus: MSEide/MSEgui, 26,107 Pascal function vectors, 768-d, L2-normalised f16
(the current on-disk form). Queries: 1,500 sampled functions, k=10,
`redundant`'s similarity gate at 0.85 (1,132 of the 1,500 sit above it).

The headline metric is **threshold flips**: how many queries change side of the
0.85 gate. Recall@10 and top-1 agreement are reported too because `misplaced`
votes over the neighbour *set* and F4 evidence names a specific neighbour.

## E1/E2 — size and fidelity ladder

| scheme | raw | zstd-19 | recall@10 | top-1 | **flips** | max cos err |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| f16 (current) | 40.10 MB | 35.99 MB | 1.0000 | 1.0000 | 0 | 0 |
| int8 per-vector scale | 20.15 MB | 17.95 MB | 0.9917 | 0.9813 | **0** | 0.0009 |
| int8 shared scale | 20.05 MB | 15.79 MB | 0.9892 | 0.9760 | **0** | 0.0018 |
| PCA384 + int8 | 11.31 MB | 8.10 MB | 0.9859 | 0.9733 | **0** | 0.0064 |
| **PCA256 + int8** | 7.58 MB | **5.89 MB** | 0.9829 | 0.9713 | **0** | 0.0134 |
| PCA128 + int8 | 3.84 MB | 3.30 MB | 0.9537 | 0.9447 | 1 | 0.0461 |
| int4 shared scale | 10.03 MB | 5.20 MB | 0.9095 | 0.8787 | 6 | 0.0736 |
| binary (sign bits) | 2.51 MB | 2.28 MB | 0.7775 | 0.7713 | 27 | 0.2697 |

For reference the file on disk today is **58.6 MB**, because the f16 payload is
base64-encoded inside JSON — a 4/3 inflation over the 40.1 MB it actually is.

int4 and sign-bit quantisation are out: they move real gate decisions.
Everything from PCA256 up is decision-preserving on this corpus.

## E4 — what a *refit* costs in git history

The first commit is not the interesting number; the recurring one is. Simulated
by mutating 5% of vectors (a refit touching 5% of functions) and measuring the
real packed size of a throwaway git repo after committing v1 then v2.

| scheme | commit 1 | after commit 2 | **cost per refit** |
| --- | ---: | ---: | ---: |
| int8 shared scale | 15.88 MB | 16.71 MB | 0.83 MB |
| PCA256 + int8, basis re-fitted | 5.22 MB | 10.46 MB | **5.23 MB** |
| **PCA256 + int8, basis frozen** | 5.22 MB | 5.47 MB | **0.25 MB** |
| PCA128 + int8, basis frozen | 2.86 MB | 3.00 MB | 0.14 MB |

**Freezing the projection basis is the whole game.** Re-fitting the basis
rotates the space, so every byte moves and each refit costs a full rewrite. Fit
the basis once, store it in the artifact, and reuse it: unchanged functions keep
byte-identical codes and git deltas collapse to the functions that actually
changed.

At 5.22 MB + 0.25 MB per refit, twenty refits cost ~10 MB of history — against a
`.git` that is already 65 MB on this repo.

A frozen basis has a second benefit: it is *computed once and committed*, so the
numerical stability of the eigendecomposition stops being a reproducibility
requirement. Only the quantisation of new vectors must be deterministic, and
rounding is.

## E3 — is an embedding reproducible? (the gate)

600 functions embedded under four configurations, raw f32 compared before any
rounding.

| pair | raw f32 identical | % components equal |
| --- | --- | ---: |
| same backend, same threads, rerun | **yes** | 100% |
| same backend, 8 vs 4 threads | **yes** | 100% |
| CPU vs GPU (Metal) | no | **0%** |

Under quantisation, CPU vs GPU agreement rises but never reaches identity:

| scheme | % components equal (CPU vs GPU) |
| --- | ---: |
| f16 | 0.54% |
| int8 per-vector | 24.5% |
| int8 shared scale | 35.8% |
| int4 shared scale | 95.4% |
| sign bits | 98.7% |

- CPU inference is **fully deterministic** — bit-identical across reruns *and*
  across thread counts, at raw f32. Nothing about parallelism threatens the
  artifact.
- CPU and GPU agree *semantically* (per-vector cosine: mean 0.99908, min
  0.99826) and disagree *numerically* everywhere.
- The disagreement is too coarse to round away: max component delta **0.0126**,
  against an int8 step of **0.00145** — 8.7× too large. Quantising hard enough
  to absorb it (int4 and below) destroys the findings, as E1/E2 shows.

**Byte-identity across machines therefore requires pinning the backend**, not
rounding harder. Pinning CPU costs the developer 3.7× on the embed pass (~19 min
instead of ~5 for this corpus, once), and removes a divergence that exists
*today and unnoticed*: a Mac fit (Metal) and a CI fit (CPU) currently produce
different semantic findings for the same commit.

**Untested and gating:** this compares ARM CPU against ARM GPU. llama.cpp
dispatches different SIMD kernels per architecture, so an x86 Linux CPU may not
match an ARM macOS CPU either. If it does not, byte-identity across contributors
is unreachable and the design must instead accept "whoever refits commits it;
the diff is noise, but rare". One Linux CI run settles it.

## E5 — does it hold as corpus size changes?

| n functions | scheme | payload | vs f16 | recall@10 | top-1 | flips | var |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000 | int8 direct | 0.77 MB | 0.50× | 0.9954 | 0.9940 | 0 | — |
| 1,000 | PCA256+int8 | 1.05 MB | 0.68× | 0.9814 | 0.9840 | 0 | 0.990 |
| 3,000 | int8 direct | 2.30 MB | 0.50× | 0.9926 | 0.9900 | 0 | — |
| 3,000 | PCA256+int8 | 1.56 MB | 0.34× | 0.9815 | 0.9825 | 0 | 0.987 |
| 10,000 | int8 direct | 7.68 MB | 0.50× | 0.9932 | 0.9938 | 0 | — |
| 10,000 | PCA256+int8 | 3.35 MB | 0.22× | 0.9813 | 0.9750 | 0 | 0.986 |
| 26,107 | int8 direct | 20.05 MB | 0.50× | 0.9905 | 0.9762 | 0 | — |
| 26,107 | PCA256+int8 | 7.47 MB | 0.19× | 0.9781 | 0.9613 | 0 | 0.986 |

- **Zero flips at every size, for every surviving scheme.** The gate decision is
  robust.
- Recall and explained variance are flat in corpus size — PCA256 holds 98.6% of
  variance whether it is fitted on 1,000 vectors or 26,000.
- The projection matrix is a fixed ~0.89 MB, so on a small corpus it costs more
  than it saves: **below ~2,000 functions int8 direct is both smaller and more
  accurate.** That is a mechanical choice at fit time ("emit the smaller
  encoding"), not a tuning knob.

## Where this lands

A committed semantic index is reachable:

- **Encoding:** PCA256 + int8 with a **frozen** basis above ~2,000 functions,
  int8 direct below. Binary payload, not base64-in-JSON.
- **Size:** 5.2 MB committed for a 26k-function repo, **0.25 MB per refit**.
- **Fidelity:** 0 of 1,132 `redundant` gate decisions changed; recall@10 0.98.
- **Reproducibility:** pin CPU inference. Same-backend output is already
  bit-identical regardless of threads.

## Still open

1. **End-to-end findings A/B.** Flips is a strong proxy, not the thing itself.
   `misplaced` votes over the neighbour set and recall@10 0.98 means ~2% of
   neighbours move. Implement the codec, rebuild an index, diff real
   `argot check` output across corpora.
2. **Cross-architecture determinism** (x86 Linux vs ARM macOS CPU). Gates the
   whole "identical for everyone" premise. One CI run.
3. **Cross-language validation.** Pascal only so far. The vector geometry is the
   model's, not the language's, so it should transfer — but it should be shown.

## Related

- `semantic-seed-cost.md` — why the 25-minute CI seed exists and why speed
  tuning cannot remove it (best realistic gain ~1.4×).
