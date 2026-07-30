# P4 — shipped static embedder: production benchmark

**Date:** 2026-07-29
**Model:** `jina-v2-code-static-256@8b0ebc376052`
**Command:** `SEM_JOBS=4 just bench-semantic`

This is the end-to-end measurement of the embedded static model through the
real `argot fit` and `argot check` paths. It supersedes transformer-era public
semantic figures. The harness fitted each corpus at its pinned base, ran the
authored reinvention fixtures, evaluated placement from the fitted index, and
replayed 150 accepted commits for raw clean-commit fires.

## Results

| Sense | Scope | Result |
| --- | --- | ---: |
| `redundant` recall | 31 corpora, 11 languages, 584 authored fixtures | **545/584 (93.3%)**; corpus range 72–100%, median 94% |
| `misplaced` recall | 22 evaluable corpora, 11 languages, 13,456 synthetic transplants | **12,899/13,456 (95.9%)**; corpus range 88–99%, median 96% |
| `redundant` raw clean-commit fires | 31 corpora, 23,766 replayed hunks | 435 (1.83%/hunk) |
| `misplaced` raw clean-commit fires | same replay | 52 (0.22%/hunk) |

Nine repositories abstained from placement because their layout had no
separable architecture: bat, commander, express, faker-js, fmt, hono, ink,
jellyfin, and rich. Abstention is intentional; it is not counted as a recall
miss.

## Precision labels are intentionally absent

Five old fire-label sets were available, but they were adjudicated against the
previous embedding model. The static model returns different nearest neighbours,
so transferring those labels would produce a false precision claim. The
consolidator refuses them and records the affected corpora in
`landing/src/data/semantic.json` as `stale_labels` until the new fire sets are
reviewed.

The generated per-corpus public table is `landing/src/data/semantic.json`; the
raw, reproducible sweep is deliberately gitignored at
`benchmarks/results/sem_all.jsonl`.
