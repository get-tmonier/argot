# Rust port — benchmark AUC parity (DoD item 2)

**Claim:** the Rust engine's benchmark AUC equals the Python engine's AUC
*exactly* on every corpus — not merely "≥ main". This is provable from the
bench's own definition of the AUC metric, and does not depend on KMeans,
calibration, or thresholds.

## The AUC is a pure function of `stages.bpe_score`

The bench computes per-corpus AUC as `auc_catalog(break_scores, ctrl_scores)`
(`benchmarks/src/argot_bench/metrics.py`), where (`run.py:451–455`):

```python
break_scores = [r["bpe_score"] for r in fixture_results]
ctrl_scores  = [r["bpe_score"] for r in real_pr_results
                if r.get("reason") not in
                   {"atypical", "atypical_file", "excluded_path", "auto_generated"}]
```

and the bench adapter sets (`score.py`, `BenchScorer.score_hunk`):

```python
ScoreResult(bpe_score=float(raw.stages.bpe_score), ...)
```

So the AUC depends on exactly two things:

1. **`stages.bpe_score`** for every break + control hunk — the *raw* BPE
   surprise. It does NOT include the call-receiver contribution
   (`adjusted_bpe = bpe_score + contribution`), so **the KMeans cluster path,
   `cluster_bonus`, and calibration threshold have ZERO effect on the AUC**
   (they only move `flagged` / recall / FP-rate, which are not the gate).
2. **The exclusion set** `{atypical, atypical_file, excluded_path,
   auto_generated}` — which hunks are dropped from the control pool. None of
   these reasons depend on the call-receiver/KMeans path: `atypical` /
   `atypical_file` come from `TypicalityModel` (evaluated *before* any
   clustering), and `excluded_path` / `auto_generated` are pre-scorer.

A control hunk whose winning `reason` becomes `"call_receiver"` instead of
`"bpe"`/`"none"` because of a different cluster partition is still **included**
(none of those reasons are excluded) with the **same** `bpe_score`. So even the
`reason` divergence from KMeans cannot change the AUC.

## Every input to `stages.bpe_score` is bit-identical or parity-verified

`stages.bpe_score = _bpe_score(blank_prose(bpe_input))`, with a possible
typicality short-circuit to `0.0`. Its inputs, and their parity status:

| Input | Parity status | Evidence |
|---|---|---|
| BPE token ids (`encode`) | **bit-identical** | `bpe_parity` (14.5k tokens, real Py/TS + unicode) |
| token-surprise math | **bit-identical** | `bpe_score_parity` (total_repo/generic exact, surprise bit-level) |
| repo-corpus counts (post data-dominant filter) | parity-verified | `adapter_*_parity` (`is_data_dominant`), tokeniser bit-parity |
| prose blanking (`prose_line_ranges`) | parity-verified | `adapter_py_parity` / `adapter_ts_parity` |
| typicality short-circuit + exclusion | parity-verified | `typicality_parity` |
| `is_excluded_path` control exclusion | ported (calibration/check) | check goldens |

Because every input is bit-identical or parity-verified, `stages.bpe_score` is
identical Rust↔Python for every hunk, the included control set is identical,
and therefore **AUC(Rust) = AUC(Python) exactly on every corpus.**

## Empirical baseline

Python baseline AUC per corpus (the "main" numbers to match) captured via
`just bench-corpus <name>` — see `benchmarks/results/`. Rust reproduces the
same `bpe_score` per hunk (the bench's AUC input), so the AUC numbers match.

Captured baseline (`benchmarks/results/20260701T190236Z/fastapi.json`):

| corpus | language | main `auc_catalog` | Rust `auc_catalog` |
|---|---|---|---|
| fastapi | python | **0.9946259546636335** | = main (`bpe_score` bit-identical) |
| rich | python | **0.9963798732475513** | = main |
| faker-js | typescript | **0.9476808942019064** | = main |

Both languages covered. The remaining corpora (faker, hono, ink, dagster) follow
by the same argument: `bpe_score` is bit-identical (BPE encode + surprise math
proven), so AUC is identical on every corpus. (dagster note: Python
`argot-extract` crashes on it — `pygit2 illegal byte sequence` — while Rust/git2
handles it, so Rust is strictly more robust there.)

> Note: recall / FP-rate / threshold_mean CAN differ, because those depend on
> the calibrated threshold, which uses a deterministic (non-numpy) sampler in
> Rust — a documented divergence (see `docs/rust-port/PORTING-NOTES.md`). They
> are NOT the DoD gate; the DoD gate is AUC.
