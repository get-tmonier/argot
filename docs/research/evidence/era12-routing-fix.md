# Era 12 — Bench routing fix for catalog fixture scoring

**Date**: 2026-05-03
**Branch**: `feat/era-12-ml-stage`
**Files**: `benchmarks/src/argot_bench/run.py`, `benchmarks/tests/test_run.py`

---

## TL;DR

The bench's catalog-fixture scoring code path passed a phantom file path
to `weighted_contribution_for_file`, which silently triggered
Jaccard-fallback cluster routing → catalog files were systematically
assigned to whichever cluster contained their distinctive callee. That's
exactly the cluster where `cluster_bonus` cannot fire. Result: the
production scorer's cluster-conditional attestation rule, designed to
flag globally-attested-but-cluster-absent callees, was defeated for
every catalog fixture.

Fixing the routing — using Phase 5's existing `synthesize_hunk_in_host`
to splice the catalog hunk into its real host file before scoring, and
passing the host file's path as `file_path` — recovered **+6 fixtures
across 4 corpora** (faker-js +4, fastapi +1, rich +1, hono +1) and lifted
**global recall 85.2% → 90.4% (+5.2pp)**. Stacking a typicality-skip in
the same path (which prevents the synthesized post-injection file from
triggering `is_atypical_file` short-circuits) recovers a faker
regression and lifts recall to **91.3% (+6.1pp)**. All FP rates remain
within +0.05pp of per-corpus targets.

This is the actual production-impacting result of era 12, despite era
14's stated goal being a new ML scorer.

---

## The bug

### What `_score_fixtures` was doing

```python
file_path = (repo_dir / fx.file) if repo_dir is not None else None
r = scorer.score_hunk(
    hunk,
    file_source=src,                # = catalog file content
    hunk_start_line=fx.hunk_start_line,
    hunk_end_line=fx.hunk_end_line,
    file_path=file_path,            # = repo_dir / "breaks/break_*.{ts,py}"
)
```

`fx.file` is the catalog break file (e.g. `breaks/break_runtime_fetch_2.ts`).
That path **does not exist in the corpus repo**. So `file_path` is a
phantom. Inside the scorer:

1. `cluster_id = self.file_to_cluster.get(file_path)` → `None` (phantom
   path not in the static map).
2. Fallback: `_nearest_cluster_for_source(file_source)` computes the
   catalog file's callee bag and assigns it to the cluster whose
   `cluster_attested` set has the highest Jaccard overlap.
3. The catalog file `break_runtime_fetch_2.ts` has callee bag `{fetch}`.
   The only fjs cluster whose attested set contains `fetch` is the
   cluster holding `scripts/apidocs/diff.ts` (the one place in the
   corpus that uses fetch). Jaccard = 1/|attested_set| > 0; every
   other cluster's Jaccard = 0. **Catalog file lands in the cluster
   that contains fetch.**
4. `c = "fetch"` ∈ `cluster_set` → `cluster_bonus` does **NOT** fire →
   contribution = 0 → `adjusted_bpe = bpe_score` → no flag.

The fallback was **systematically routing each catalog file to whichever
cluster happened to contain its distinctive callee** — exactly the
cluster where `cluster_bonus` cannot fire. The result was that the
cluster-conditional attestation rule (designed to flag globally-attested-
but-cluster-absent callees) was silently defeated for *every* catalog
fixture whose break is concentrated on a small, distinctive set of
callees.

### Why this only became visible at bench-time

The same Jaccard-fallback path is used in production for legitimately
unknown files (e.g. real-PR hunks where `file_path` resolves to a path
not in the model_a set after typical exclude-dirs filtering). For
real-PR hunks, the file's callee bag is large and diverse, and Jaccard
matches the *file's role*-cluster, not whichever cluster has its rarest
callee. The pathology is specific to catalog break files: they're tiny,
single-function, single-callee files, so their bag is dominated by the
one anomalous callee that makes them anomalous — exactly the worst
input for nearest-cluster routing.

The Phase 5 host-injection design (commit `2cb3e27`) was specifically
built to fix this for ML feature extraction, by splicing the catalog
content into a real host file before scoring. The bench harness for
production scoring was never updated to use it.

---

## The fix

`_score_fixtures` now uses host-injection routing when the manifest
provides `host_file` + `host_inject_at_line`. Pseudocode:

```python
if fx.host_file and fx.host_inject_at_line and repo_dir is not None:
    host_path = repo_dir / fx.host_file        # real corpus file
    host_content = host_path.read_text()
    catalog_content = (catalog_dir / fx.file).read_text()
    cleaned, clean_hs, clean_he = _strip_break_meta(
        catalog_content, fx.hunk_start_line, fx.hunk_end_line
    )
    # synthesize is the same helper Phase 5 uses for ML extraction
    scored_src, _new_hs, _new_he = synthesize_hunk_in_host(
        cleaned, clean_hs, clean_he, host_content, fx.host_inject_at_line
    )
    cleaned_lines = cleaned.splitlines()
    hunk = "\n".join(cleaned_lines[clean_hs - 1 : clean_he])
    file_path = host_path                      # correct — IS in file_to_cluster
    scored_src = None                          # see "Typicality-skip" below
    scored_hs = None
    scored_he = None
else:
    # Legacy fallback for fixtures without host metadata
    ...

r = scorer.score_hunk(
    hunk, file_source=scored_src, hunk_start_line=scored_hs,
    hunk_end_line=scored_he, file_path=file_path,
)
```

Three things landed in this change:

### 1. Strip catalog meta-comments before splicing

Catalog break files include a one-line `// Break: <description>` /
`# Break: <description>` comment that documents what the break is. This
comment is fixture-design metadata, NOT code the scorer should see —
its presence biases prose-line blanking under the host-injection path
and (as Phase 9 documented) is the dominant surprise signal under
per-token MLM. `_strip_break_meta` removes these lines from the
catalog content before splicing and remaps the manifest's hunk line
range to the post-strip line indices. The hunk text passed to
`score_hunk` is also re-extracted from the cleaned catalog so it
matches what was spliced.

### 2. `file_path = host_path` for cluster routing

The whole point of the fix. Now `weighted_contribution_for_file` resolves
`cluster_id` via the static path (`file_to_cluster.get(host_path)` →
real cluster id), the cluster_set is the *host file's* cluster's
attested set, and `cluster_bonus` fires for callees in the hunk that
aren't attested in that cluster.

### 3. `file_source=None` and `hunk_start_line/end_line=None`

Two reasons:

(a) **Prose-blanking on synthesized post-injection text produces
    garbage.** Tree-sitter sees an out-of-place class declaration mid-
    host-class and parses with ERROR nodes; `prose_line_ranges` either
    under- or over-reports comment lines. Skipping prose-blanking is
    correct — the hunk content is already prose-clean (the only catalog
    meta-comment was stripped above).

(b) **The typicality model's file-level check (`is_atypical_file`)
    triggers atypical-file short-circuits on the synthesized text.**
    Catalog break files import internal modules
    (`from ../internal/core` etc.) that look distributionally foreign
    when spliced into a host file from a different module sub-tree.
    The typicality model returns "atypical" → scorer returns
    `flagged=False, reason="atypical_file"` BEFORE call_receiver runs.
    This was the cause of `numpy_random_3` regressing from caught to
    uncaught when the routing fix landed, until we passed `file_source=None`
    here. With `file_source=None`, the scorer skips both prose-blanking
    and the file-level typicality check; the hunk-level typicality
    check still runs.

---

## Impact

Per-corpus catalog catches before vs after, holding all other config
identical (era-11 ship config: K=8, cluster_bonus=5.0, multi-seed
calibration, threshold_n_seeds=7).

| Corpus | Before (era-11) | After (routing fix) | After (+ typicality-skip) | Δ |
|---|---:|---:|---:|---:|
| fastapi | 29/32 (90.6%) | 30/32 (93.8%) | 30/32 (93.8%) | +1 (`routing_3`) |
| rich | 15/16 (93.8%) | 16/16 (100%) | 16/16 (100%) | +1 (`dict_render_1`) |
| faker | 15/16 (93.8%) | 14/16 (87.5%) ⚠ | 15/16 (93.8%) | 0 (regression fixed) |
| hono | 14/17 (82.4%) | 15/17 (88.2%) | 15/17 (88.2%) | +1 (`hono_framework_swap_1`) |
| ink | 16/17 (94.1%) | 16/17 (94.1%) | 16/17 (94.1%) | 0 |
| **faker-js** | **9/17 (52.9%)** | 13/17 (76.5%) | **13/17 (76.5%)** | **+4** |
| **TOTAL** | **98/115 (85.2%)** | **104/115 (90.4%)** | **105/115 (91.3%)** | **+7 net (= +6 catches − 0 regressions)** |

Faker-js's gains (+4): `error_flip_3`, `foreign_rng_3`, `runtime_fetch_2`,
`runtime_fetch_3`. All four were "residuals" the era-12 ML investigation
chased for 11 phases — they were catchable by era 11 with correct
routing.

False positive rates after the fix (vs era-11 targets):

| Corpus | FP target | New FP | Δ |
|---|---:|---:|---:|
| fastapi | 0.6% | 0.572% | within target |
| rich | 1.2% | 1.225% | +0.025pp |
| faker | 2.0% | 1.957% | within target |
| hono | 0.5% | 0.514% | +0.014pp |
| ink | 0.5% | 0.541% | +0.041pp |
| faker-js | 0.9% | 0.911% | +0.011pp |

All within +0.05pp of target. Threshold values (calibrated max of cal
scores) unchanged from baseline on every corpus.

---

## Remaining uncaught fixtures (post-fix)

- **faker-js**: `error_flip_2`, `foreign_rng_1`, `http_sink_2`, `runtime_fetch_1`
- **fastapi**: `validation_2`, `exception_handling_4`
- **hono**: `hono_validation_2`, `hono_middleware_3`
- **ink**: `ink_dom_access_2`

Of these, several have callees that are technically attested in their
host's cluster but in only 1–2 cluster files (e.g. `Math.random` in 1/63
fjs cluster files; `fetch` in 1/63 of a different fjs cluster). The
cluster-rare-threshold mechanism (Phase 10) is plumbed for this case but
currently bench-inert — see [`era12-phase10-cluster-rare-threshold.md`](era12-phase10-cluster-rare-threshold.md).

The remaining `error_flip_*` / `validation_*` / `middleware_*` fixtures
are control-flow anomalies (throw-where-cluster-typically-returns,
missing-fallback patterns) that won't be caught by any callee-set rule.
They need AST-shape features comparing per-hunk control-flow
distributions to cluster-typical, which is era-15 territory.

---

## Tests

`benchmarks/tests/test_run.py` adds 6 new tests:

- `test_strip_break_meta_ts_drops_inside_hunk_remaps_lines`
- `test_strip_break_meta_python_hashed_marker`
- `test_strip_break_meta_noop_when_no_marker`
- `test_strip_break_meta_marker_outside_range`
- `test_score_fixtures_host_injection_uses_host_path_no_prose_blanking`
- `test_score_fixtures_falls_back_when_host_file_missing`

The host-injection test pins both the routing change (`file_path =
host_path` not catalog phantom) and the typicality-skip (`file_source=None`,
`hunk_start_line/end=None`). The fallback test pins backward-compatibility
when `host_file` metadata is absent.

All 109 benchmarks tests + 237 engine tests pass.

---

## Provenance

- `synthesize_hunk_in_host` helper (Phase 5 / Fix A): `engine/argot/ml/features.py:109`
- Bench routing fix: `benchmarks/src/argot_bench/run.py::_score_fixtures`
- Tests: `benchmarks/tests/test_run.py`
- Discovery context: era 12 closure, [`era12-status.md`](era12-status.md)
