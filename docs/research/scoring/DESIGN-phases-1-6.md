# Scoring Benchmark & Technique Research — Design

> **Scope**: Phases 1–6 (TF-IDF era). For Phase 7 onwards see [`DESIGN-phase-7.md`](DESIGN-phase-7.md).

**Status**: approved 2026-04-18
**Branch**: `research/scoring-benchmark`
**Tracker**: [`ROADMAP.md`](ROADMAP.md) (read this at the start of every session)

## Why

`argot check` currently produces scores, but we don't know how well the
underlying model actually performs, or how much training data a repo needs
before it does. The diagnostic harness landed in branch
`feat/scoring-diagnostic-harness` surfaced two findings:

- On argot's own history (127 records) the model is effectively random
  (AUC 0.52 vs shuffled tokens).
- On a larger cross-repo corpus (~5,200 records) AUC is 0.62-0.66 — better
  than random but still weak.

Before we tune thresholds or touch features, we need a rigorous picture of:

1. **How does model quality (AUC) scale with dataset size?** → tells users
   whether argot is a fit for their repo.
2. **Which training changes lift AUC the most per unit of engineering
   effort?** → tells us where to invest.

This research produces that picture, documented in-repo as the authoritative
"here's what argot's scoring can and can't do" reference.

## Scope

In-scope:
- A reproducible benchmark harness that runs `validate` across a fixed
  corpus of OSS repos.
- A sizing study: AUC vs dataset size across 5 buckets (micro → xlarge).
- Six named technique experiments: `context_after`, `embed_dim`, `epochs`,
  `char n-grams`, `imports-in-scope`, `file-path embedding`.
- Documentation of all findings in `docs/research/scoring/` (per-experiment
  logs) and `docs/scoring.md` (evergreen summary).
- A roadmap that survives across sessions.

Out of scope (for this branch):
- Any threshold recalibration (separate proposal in
  `~/.claude/plans/scoring-fix-proposal.md`, deferred).
- Shipping model changes to users by default (experiments are measurements;
  integration happens in follow-up branches).
- CLI surface changes.

## Design

### Phase 1 — benchmark infrastructure

Three additions to the engine:

**1a. Repo tagging in `extract`**

Add `--repo-name <slug>` to `argot-engine extract`. Stamps each record with
`_repo: <slug>`. Default value derived from the git repo's origin URL basename
when not specified.

Rationale: `validate` already has cross-repo AUC logic
(`validate.py:145-157`) gated on records carrying a `_repo` field. Today,
`extract` doesn't set it, so that code path never activates.

**1b. Dataset concatenation utility**

New command `argot-engine corpus concat <in1.jsonl> <in2.jsonl>... -o <out.jsonl>`.

Responsibilities:
- Concatenate all input JSONLs into one output.
- Verify each record has a `_repo` tag; error if not.
- Print a summary: total records, records per `_repo`.

Does nothing fancy. Guardrail against silently producing untagged combined
datasets.

**1c. Batch benchmark runner**

New command `argot-engine corpus benchmark`:

```
argot-engine corpus benchmark \
  --dataset <combined.jsonl> \
  --sizes 500,2000,8000,32000 \
  --seeds 3 \
  --out .argot/research/results.jsonl
```

For each `(size, seed)` pair:
- Deterministic downsample of the dataset to `size` records (stratified by
  `_repo` to preserve the home/foreign split).
- Run the existing `validate` logic (train on 80%, score held-out + three
  adversarial sets).
- Append one row to the output JSONL:
  ```json
  {"size": 2000, "seed": 0, "n_repos": 3,
   "shuffled_auc": 0.58, "cross_auc": 0.60, "injected_auc": 0.58,
   "good_median": 0.42, "good_p95": 1.85, "trained_at": "2026-04-18T..."}
  ```

Re-runs are append-only; the results file is an accumulating experiment log.

Does NOT modify the existing `validate` command — batch runner composes it.

### Phase 2 — sizing study

**Corpus** (2 repos per bucket; TS + Py mix):

Repos are classified by **extracted record count** — the number of commit
records argot's extractor produces from the full git history. Bucket targets
are set to match the available scale of each pair.

| bucket  | target records | TS repo                  | Py repo                    | records (approx)   |
|:--------|:---------------|:-------------------------|:---------------------------|:-------------------|
| micro   | ~250           | argot (self, ts+py)      | —                          | 243                |
| small   | ~3,000         | TBD (~2–4k)              | `astral-sh/ruff` v0.1.0    | TBD + 3,343        |
| medium  | ~7,000         | `vitejs/vite` v2.0.0     | `pallets/click`            | 8,252 + 6,334      |
| large   | ~20,000        | `Effect-TS/effect`       | `pydantic/pydantic`        | 21,693 + 27,787    |
| xlarge  | ~60,000        | `microsoft/vscode`       | `django/django`            | TBD + 174,877      |

Final URLs, SHAs, and exact record counts are pinned in `01-corpus.md`.
Swap-outs and drops are logged in `01-corpus.md §Swap-out log`.

**Protocol per bucket:**
- Clone each repo at the pinned commit into `.argot/research/repos/<slug>/`.
- Extract with `--repo-name <slug>`. Cache `.argot/research/datasets/<slug>.jsonl`.
- For each bucket: concat the two repos' datasets, run
  `argot-engine corpus benchmark` at the bucket's target size with 3 seeds.
- Append to `results.jsonl`.

**Output**: Phase 2 closes with `docs/research/scoring/02-sizing-study.md`
containing the AUC-vs-size table, a short interpretation, and the
"minimum-viable corpus size" finding (the smallest bucket where all three
AUCs consistently clear 0.7).

### Phase 3 — technique experiments

Each technique is a separate branch off `research/scoring-benchmark`. Protocol:

1. Implement the change (behind a flag or env var; no default shift).
2. Re-run the Phase 2 corpus benchmark on the variant.
3. Compute AUC delta per bucket vs baseline.
4. Write `docs/research/scoring/<nn>-<technique>.md`:
   - Hypothesis
   - What changed
   - Results table (baseline vs variant across buckets)
   - Interpretation (did it help, at what size did it start helping, any regressions)
5. Merge docs regardless of outcome. Merge code only if the lift warrants it.

Techniques, ordered by expected impact-per-effort:

| # | id              | one-line                                            | effort |
|--:|:----------------|:----------------------------------------------------|:-------|
| 1 | `context_after` | Wire the already-extracted field into training      | S      |
| 2 | `embed_dim`     | 128 → 256; re-benchmark                             | XS     |
| 3 | `epochs`        | 50 → 200; re-benchmark                              | XS     |
| 4 | `char_ngrams`   | Add character n-grams to TF-IDF vectorizer          | S      |
| 5 | `imports_scope` | Extract top-level imports as a separate signal      | M      |
| 6 | `path_embed`    | Embed file path; concatenate to context encoder     | M      |

Stop rule: if a technique's lift is < 0.01 AUC across all buckets, we note it
and move on — no integration to `main`.

### Phase 4 — documentation (runs throughout)

Structure:

```
docs/
├── scoring.md                              # evergreen summary (user-facing)
└── research/
    └── scoring/
        ├── DESIGN.md                       # this file
        ├── ROADMAP.md                      # living status tracker
        ├── 01-corpus.md                    # Phase 2 repo list + pins
        ├── 02-sizing-study.md              # Phase 2 results
        ├── 03-context-after.md             # Phase 3 #1
        ├── 04-embed-dim.md
        ├── ...
        └── 99-synthesis.md                 # final writeup
```

Per-experiment logs stay append-only. `docs/scoring.md` gets overwritten with
the latest findings as we learn more (with a "last updated" date).

## Working across sessions

The roadmap is authoritative state. Every session starts and ends with it.

- **At session start**: read `ROADMAP.md`. It lists the current phase, what
  is done, what is next, and any open questions parked from the last session.
- **At session end**: update `ROADMAP.md` with progress made, decisions
  taken, questions surfaced, next action to pick up.
- `ROADMAP.md` is terse (bullet lists, ~100 lines max). The detailed
  findings live in per-experiment docs.

## Testing

- Phase 1 code is production-path; gets the usual mypy + ruff + pytest
  treatment. Unit tests for the downsample/stratify logic and the concat
  utility (both are pure functions with clear contracts).
- Phase 2/3 do not require unit tests — they ARE tests, measuring the model.
- Every Phase 1 PR passes `just verify`.
- Research PRs (Phase 2 & 3) need not pass `just verify` for the docs
  changes, but any code change piggybacking on them must.

## Success criteria

- A single command (`just research benchmark`) produces the full
  `results.jsonl` from a clean checkout + repo clones.
- `docs/scoring.md` answers "how much git history do I need?" with a number.
- Each of the 6 techniques has a documented outcome (helped / didn't /
  inconclusive) with numeric support.
- An outside reader with zero context can understand what was tried, what
  worked, what didn't, by reading the `docs/research/scoring/` folder in order.
