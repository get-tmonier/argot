# Per-language calibration + streaming corpus scorer

> **TL;DR.** Argot now calibrates one threshold per language present in
> the repo, persisted in a v2 `scorer-config.json` keyed by language.
> `argot-check` dispatches each hunk's file extension to the matching
> scorer at score time. The change ships with an engine streaming
> refactor — `SequentialImportBpeScorer` no longer materialises per-file
> content, so calibration scales to monorepo-class corpora (Dagster,
> ~6000 files in one tree) without OOM. Bench picks up Dagster as a
> reference multi-language corpus and runs end-to-end in ~28 min on the
> full corpus.

## Context

argot's pre-existing calibration produced **one** threshold per repo
from a single mixed-language corpus. On a single-language repo (faker,
fastapi, hono, …) this is fine — the joint distribution **is** the
language's distribution. On a true monorepo (argot itself: TypeScript
CLI + Python engine; Dagster: Python `python_modules/` + TypeScript
`js_modules/`) the joint threshold is whatever the mixed token
distribution settles on, which isn't meaningful for *either* language.

Symptom: out-of-the-box `argot check` on argot itself missed anomalies
that would fire if calibrated per-language, because the joint threshold
was dominated by the language with broader token diversity.

A backlog PRD existed (`multi-language-corpus`) documenting three
candidate designs:

1. Per-language threshold only — single corpus, one threshold per
   language. Smallest change.
2. Per-language corpora — split the file list per language, train and
   calibrate independently. Bigger but the honest version.
3. Activation gate — only kick the per-language path in when minority
   language exceeds 10% of files.

This PR closed those open decisions and shipped the work.

## Decision

### Calibration shape

**Per-language calibration end-to-end** (option 2). One scorer per
language present, each with its own threshold + evidence corpus +
import surface + clusters. No activation gate — single-language repos
just emit one entry under the `languages` map.

Each scorer is independent:

- BPE generic baseline built only over that language's source files
- `import_modules` set built only over that language's imports
- Call-receiver clusters built only over that language's files
- Calibration threshold sampled only over that language's hunks

The honest design — Python and TypeScript have genuinely different
token distributions, and conflating them was the symptom that surfaced
this work in the first place.

### Schema

`scorer-config.json` is reshaped to v2:

```json
{
  "version": 2,
  "languages": {
    "python": {
      "threshold": 4.82,
      "call_receiver_alpha": 2.0,
      "import_modules": ["argparse", "json", "..."],
      "evidence_corpus": { ... },
      "calibration": { "n_cal": 500, "seed": 0, "n_seeds": 3, ... }
    },
    "typescript": { /* same shape */ }
  }
}
```

v1 configs (single-language flat) crash at load with a clear
"regenerate via `argot-calibrate`" message. **No migration code.** No
backward-compat shim. argot is pre-prod (no real users); a clean
reshape beats a compat carrying-cost.

### Activation rule

`argot-calibrate` partitions `repo-corpus.txt` by file extension into
language buckets, then runs the calibration loop once per non-empty
bucket. No CLI flags; no thresholds. Same call invocation works for
single-language and multi-language repos.

`argot-check` loads the v2 config, instantiates one scorer per
`languages[*]` entry, and routes each hunk by file extension at score
time (`.py` → python, `.ts/.tsx/.js/.jsx` → typescript). `.js`/`.jsx`
fallback to TypeScript for the same reason TypeScript's adapter
already covers them.

### Catalog manifest

For multi-language bench corpora, the catalog manifest gets a per-
fixture `language` field:

```yaml
corpus: dagster
language: multi
fixtures:
- id: dagster_py_framework_swap_1
  language: python
  ...
- id: dagster_ts_framework_swap_1
  language: typescript
  ...
```

Single-language catalogs (the existing 6) are unchanged — the field
falls back to the catalog top-level `language`. Dagster ships as the
first multi-language corpus, with 24 hand-crafted fixtures (12 Python
+ 12 TypeScript) targeting Dagit (UI) and Dagster core idioms.

The bench reports two rows per multi corpus (`dagster (python)` and
`dagster (typescript)`), each with the same metric columns as a
single-language row.

## Why not the alternatives

**Per-language threshold only (option 1)** — share BPE generic baseline
and import surface across languages, only split the threshold. Smaller
change, but the joint BPE baseline still dominates token frequencies
with the larger language, and the joint import surface mis-classifies
imports. Threshold alone doesn't move the needle if the underlying
distributions stay mixed.

**Activation gate (option 3)** — only per-language when minority lang
≥ 10% of files. Saves complexity for true single-language repos but
adds a gate condition to maintain. Per-language with one entry under
`languages` for single-language repos is a uniform code path — same
work either way, no special case to break later.

**Keep v1 schema, add `per_language: { ... }` block** — backward-compat
shim. Pre-prod policy says clean reshape over compat (`feedback_pre
prod_strong_architecture` in MEMORY); a v1 reader living forever for
zero users isn't worth the code mass. v2-only with a clear regenerate
message is the right pre-prod move.

**Sample the corpus to bound memory** — was the bench's first attempt;
gave wrong calibration thresholds because rare internal modules were
silently missed. Streaming the scorer is the principled fix; sampling
hides production bugs from the bench. (See [Streaming corpus
scorer](#streaming-corpus-scorer) below.)

## Streaming corpus scorer

Calibration on Dagster (5950 Python files in one tree) initially
peaked at 50 GB RAM before getting SIGKILL'd. The scorer was
materialising per-file content during BPE baseline and clustering
passes.

**Refactor**: `SequentialImportBpeScorer.__init__` now accepts an
`Iterable[Path]`; each pass over the file list reads one file at a
time, accumulates into bounded data structures (Counter, set,
per-file MinHash signature), and drops the content. Working-set on
Dagster sits under 4 GB.

The bench's earlier file-list reservoir cap (3000 files per language)
is **gone** — it was masking real production behaviour. The cap not
only reduced memory; it also produced **biased thresholds**: on Dagster
Python, the 3000-file cap calibrated to threshold 9.94 (recall 33%
across the catalog), whereas full-corpus calibration produces
threshold 4.82 (recall 100%). Sampling was hiding ~67% of the bench
signal.

The streaming refactor is what makes the per-language design viable
at scale. Without it, "calibrate one scorer per language" doubles the
memory cost of a non-trivial repo and breaks any monorepo user.

## Bench wall-time tuning

While shipping the streaming refactor, three accumulated measurement
costs were dropped — they were validating properties already pinned
in production code:

- `cfg.seeds` default `[0,1,2,3,4]` → `[0]`. Outer-seed multiplicity
  was era-7 stability infrastructure; only `seeds[0]` actually scored
  anything (the rest populated the `calibration_stability` metric).
  Multi-seed median is run *internally* per build (see below); the
  outer loop was redundant. `--seeds N` opts back in for users who
  want the stability metric.
- `threshold_n_seeds` default `7` → `3`. Era-10 shipping config was
  set when each scorer build was a few seconds; on monorepo corpora
  each build is tens of seconds and the multi-seed cost dominates.
  Median converges within 0.01 of N=7 at N=3 across the existing 6
  corpora.
- `argot-extract` per-PR output is now bounded by `--limit 10000` (the
  bench's `ensure_extracted` passes the flag). Previously each PR's
  `dataset.jsonl` walked full git history; on Dagster a single PR
  emitted 429k records (~16 GB JSONL). Bench then re-read that file
  multiple times across the seed loop. The downstream consumer is
  also streaming + projects per-record to drop the bulky token
  arrays — see `_real_pr_hunks` and `_partition_real_pr_hunks_by_lang`.

These are accumulated debt from prior eras; their measurement
properties have all been validated and shipped. Result: full Dagster
bench in ~28 min, vs the ~4-hour estimate without the cuts (or never
finishing on 50 GB OOM before the streaming refactor).

## Consequences

**User-visible**:

- `argot check` on a monorepo now scores Python and TypeScript hunks
  against their own thresholds. Previously joint threshold dominated.
- `argot calibrate` works on monorepo-class repos without OOM. Memory
  scales with vocabulary / cluster count, not file count.
- v2 scorer-config.json — anyone with a saved v1 config has to
  regenerate (one command, deterministic output).

**Bench**:

- Dagster pinned in `targets.yaml` as the first `language: multi`
  corpus. 24 fixtures, 5 PR snapshots.
- Two rows in bench report per multi corpus: `dagster (python)` and
  `dagster (typescript)`.
- New defaults for outer seeds + multi-seed-N are opt-out via CLI
  flags. Existing 6 corpora's metrics are byte-stable on the same
  seed (verified pre/post).

**Engine surface**:

- `SequentialImportBpeScorer.__init__` accepts `Iterable[Path]` (was
  `list[Path]`). Existing callers passing lists continue to work.
- `engine/argot/scoring/calibration/sampling.py` was briefly added then
  removed once production decided not to ship a corpus cap.

**Documentation**:

- `engine/CONTEXT.md` updated with the streaming-iterator note.
- `benchmarks/README.md` (TODO once full bench numbers settle) gains
  the dagster entry + the multi-language headline.

**Known follow-ups** (issues filed separately):

- Bench cleanup beyond the seed/n_seeds defaults — reuse primary
  scorer across PR snapshots (skip the secondary-PR rebuild loop) is
  the single biggest remaining lever, ~10× speedup on Dagster.
- Per-PR scorer rebuild for control hunks — currently each non-primary
  PR triggers a fresh full calibration. The intent was to "match the
  voice at this PR's SHA"; needs validation that thresholds don't
  shift meaningfully across 5 recent PRs.
- Bench's per-PR extract `--limit` tuning — 10000 was picked by
  argument; could be principled per-corpus.
- `auto_select_asym_cal` probe currently runs once per `build_scorer`
  call. Could be cached per (corpus, language) since the fire rate is
  a property of the corpus, not the seed.

These are tracked in `.scratch/` issue files and the
`feat/multi-language-calibration` branch's PR description.
