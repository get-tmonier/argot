# AST contrastive treelets: the click rerun and abandonment

## Setup

The click corpus was Phase 13's second cross-domain test for
`ContrastiveAstTreeletScorer`. The initial run used a 6-file `model_A`
and whole-file controls, and came in at AUC 0.1187 — a catastrophic
failure, but the methodology was open to challenge on two axes: corpus
starvation (too few `model_A` files) and hunk-size mismatch (breaks were
~20-line hunks, controls were whole files). A matched rerun was ordered
to settle whether the scorer was FastAPI-tuned or simply under-fed.

The matched rerun used a larger `model_A` — 13 click source files (all
of `click/*.py` minus three held-out files: `decorators.py`, `types.py`,
`core.py`) — and switched controls to 10 × ~20-line hunks drawn from the
three held-out files — size-matched controls to the break hunks. Fixtures
covered `argparse_class`, `optparse`, `raw_argv`, and `docopt` style
breaks against click's idiomatic decorator-based CLI. Pre-registered
FAIL conditions: AUC < 0.35 *and* Δ < 0.30 over the initial run.

## Results

Matched-rerun AUC **0.2500**, up from **0.1187** (Δ = +0.1313). Both
FAIL conditions met.

| run | model_A | controls | AUC |
|:---|:---|:---|---:|
| v1 initial | 6 files | whole-file | 0.1187 |
| v2 matched rerun | 13 files | size-matched ~20-line hunks | 0.2500 |

Median control score was 7.1494; the best break category (`optparse`)
scored 7.1610 — indistinguishable. `argparse_class` breaks landed at
5.1160–6.0835, below most controls. No category ranked above the bulk
of controls. `control_decorators_command` and `control_decorators_option`
scored 8.0561 — a corpus-gap artefact from holding out `decorators.py`,
which pushed controls *higher* than breaks.

## Interpretation

Both methodology fixes were granted, and the scorer still failed —
abandonment upheld. The verdict is that the scorer is fragile to
corpus size: on small or lexically-generic repos, control hunks score
as anomalously as the breaks because the axis conflates "unusual for
the stdlib" with "unusual for the repo", and those two things only
coincide in special cases like FastAPI. Two independent cross-domain
failures (rich and click) closed the AST-contrastive thread and
redirected Phase 14 toward the import-graph idea.

---

*Source on tag `research/phase-14-pre-cleanup`:
`docs/research/scoring/signal/phase13/stage3_tier3_click_matched_2026-04-21.md`.
Re-written here for clarity, not copied.*
