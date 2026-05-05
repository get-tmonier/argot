# Engine Context

The Python scoring pipeline that learns a repo's voice from its git history and flags hunks whose token distribution diverges from the learned norm.

## Language

### Core concepts

**Voice**:
The observable style patterns of a repo — its idiomatic token choices, call conventions, and import vocabulary.
_Avoid_: style, patterns, conventions

**Hunk**:
The unit the scorer operates on — a diff slice (during `check`) or a sampled top-level function/class (during `calibrate`). Calibration hunks are *synthetic* (sampled from the live tree, not from git history).
_Avoid_: chunk, block, snippet

**Attested**:
Present in the repo's own corpus — said of a callee or import that appears in the repo's source files.
_Avoid_: native, known, seen

**Voice profile**:
The set of fit artifacts in `.argot/` that encode the repo's learned voice: `dataset.jsonl`, `repo-corpus.txt`, `generic-baseline.json`, `scorer-config.json`.
_Avoid_: model, artifacts, trained state

### Scoring

**Repo corpus**:
The repo's own token frequency distribution, built from its non-test source files. One input to the BPE scorer. Persisted as `.argot/repo-corpus.txt`.
_Avoid_: model A, model_a

**Generic baseline**:
The pre-built open-source token frequency distribution bundled with argot. The other input to the BPE scorer. Persisted as `.argot/generic-baseline.json`.
_Avoid_: model B, model_b, BPE reference

**Surprise score**:
The max BPE log-likelihood ratio over a hunk's tokens — how different the hunk's token distribution is from the repo corpus relative to the generic baseline. The BPE scorer's primary output.
_Avoid_: score, BPE score, log-ratio

**Cluster**:
A group of source files with similar callee bags, derived by MinHash + KMeans at fit time (K=8). Used to detect context-dependent breaks — a callee attested globally but absent from its file's cluster.
_Avoid_: file group, archetype, category

### Scorer steps

**Typicality filter**:
The pre-scorer gate that skips hunks whose content is structurally data-dominant (high literal-leaf ratio) or whose enclosing file is globally data-dominant.
_Avoid_: pre-scorer, Stage 0, atypicality filter

**Import checker**:
The first scorer step — flags a hunk immediately if any of its imports are unattested in the repo.
_Avoid_: Stage 1, import graph

**BPE scorer**:
The second scorer step — computes the surprise score and applies the call-receiver penalty to produce an adjusted score compared against the calibration threshold.
_Avoid_: Stage 2, BPE stage

## Relationships

- The **pipeline** produces a **voice profile** by fitting the scorer to a repo's git history
- A **hunk** is scored by the **typicality filter** → **import checker** → **BPE scorer**, in that order
- The **BPE scorer** compares the **repo corpus** against the **generic baseline** to compute the **surprise score**
- **Clusters** are computed at fit time; each **hunk** is scored against its file's cluster's attested callee set
- A hunk is flagged if the **import checker** fires or the **BPE scorer**'s adjusted score exceeds the calibration threshold

## Example dialogue

> **Dev:** "If a callee is attested globally but never appears in files like this one, should we penalize it?"
> **Domain expert:** "Yes — that's exactly what clusters are for. A callee can be globally attested but still wrong for this file's context. The cluster tells us which callees are normal *for this kind of file*."

## Scorer fit: streaming iterator

`SequentialImportBpeScorer` (and its inner `CallReceiverScorer`) accepts an
`Iterable[Path]` for the repo corpus so calibration scales to monorepo-class
corpora without sampling.  Internally, `CallReceiverScorer.__init__` makes a
single pass over the file list: it builds the global attested-callee set,
per-file callee bags (frozensets), and 128-element MinHash signatures together.
After clustering the signatures with KMeans, `cluster_attested` and
`cluster_callee_counts` are computed from the bags already in memory — no
second tree-sitter pass.  The bags are then explicitly freed; peak working-set
from that point is bounded by signature storage (O(n_files × 128 ints)).  A
separate file-read pass runs only when shape primitives are registered (an
optional bench feature), and not for the common production path.  This is what
allows a ~6 000-file corpus like Dagster to calibrate cleanly inside 4 GB.

## Flagged ambiguities

- "model A / model B" — resolved: **repo corpus** and **generic baseline** respectively. The on-disk filenames (`.argot/repo-corpus.txt`, `.argot/generic-baseline.json`) match the domain language directly; nothing in production code or fixtures still carries the `model_a`/`model_b` identifier.
- "native" (used once in README for imports) — resolved: **attested** is canonical.
- "Stage 1 / Stage 2 / Stage 1.5" — resolved: use **typicality filter**, **import checker**, **BPE scorer**. Stage numbers are not domain language.
