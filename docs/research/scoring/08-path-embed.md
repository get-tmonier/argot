# Technique 6 — path_embed (file path embedding)

**Branch**: `research/path-embed`  
**Status**: complete 2026-04-19  
**Baseline**: [`02-sizing-study.md`](02-sizing-study.md)

## Hypothesis

File paths carry structural style signals: which directories a repo uses (`src`,
`packages`, `tests`, `internal`), naming conventions in filenames (camelCase vs
snake_case), and file extensions (`.ts` vs `.py`). Embedding the file path as
a separate TF-IDF feature vector and concatenating it to the main context/hunk
representation should give the model a cheap structural signal that doesn't
require reading any code.

## What changed

`train.py` gains `_path_to_text(file_path)`: splits a path on `/`, `.`, `_`,
`-` into space-separated tokens. `packages/ai/openai/src/OpenAiLanguageModel.ts`
→ `"packages ai openai src OpenAiLanguageModel ts"`. This turns path components
and file extensions into bag-of-words features.

`train_model` gains `path_embed: bool = False`. When True, fits a
`TfidfVectorizer(max_features=200)` on path texts per record, then concatenates
path features to both `ctx_x` and `hunk_x`. `ModelBundle` gains
`path_vectorizer: TfidfVectorizer | None = None`.

`_load_records` gains `path_embed=False`: when True, stores `_file_path` from
the raw record before slimming. `_file_path` survives `shuffle_negatives` and
`inject_foreign` via `{**r, ...}` spread.

`score_records` in `validate.py` checks `bundle.path_vectorizer`: if set,
extracts `r.get("_file_path", "")` and concatenates path features.

The CLI gains `--path-embed`. The justfile gains
`research-benchmark-path-embed`.

## Results

Baseline is Phase 2 mean ± std (3 seeds). Variant: 3 seeds,
small/medium/large only.

### Shuffled AUC

| bucket | baseline       | variant        | Δ       |
|:-------|:---------------|:---------------|:--------|
| small  | 0.500 ± 0.000  | 0.505 ± 0.002  | +0.005  |
| medium | 0.501 ± 0.000  | 0.634 ± 0.023  | **+0.132** |
| large  | 0.637 ± 0.005  | 0.610 ± 0.009  | **−0.027** |

### Cross-repo AUC

| bucket | baseline       | variant        | Δ       |
|:-------|:---------------|:---------------|:--------|
| small  | 0.446 ± 0.023  | 0.867 ± 0.006  | **+0.421** |
| medium | 0.395 ± 0.004  | 0.709 ± 0.038  | **+0.315** |
| large  | 0.544 ± 0.012  | 0.709 ± 0.006  | **+0.164** |

### Injected AUC

| bucket | baseline       | variant        | Δ       |
|:-------|:---------------|:---------------|:--------|
| small  | 0.450 ± 0.017  | 0.712 ± 0.020  | **+0.262** |
| medium | 0.398 ± 0.009  | 0.662 ± 0.022  | **+0.264** |
| large  | 0.631 ± 0.008  | 0.615 ± 0.007  | −0.016  |

## Interpretation

**Path features produce the largest cross-repo gains of any technique
(+0.421 at small, cross at 0.867) but show the same repo-identity pattern as
`imports_scope`: the gains come from fingerprinting repos, not learning style.**

### Cross-repo: strongest signal in the entire experiment series

Cross at small reaches 0.867 — the highest number produced by any technique.
The mechanism is clear: effect stores all source under `packages/*/src/*.ts`
while pydantic stores tests under `tests/test_*.py`. A 200-feature path TF-IDF
almost perfectly separates the two repos by directory structure alone, without
reading a single code token. At large (20k records), cross reaches 0.709 — well
above the baseline and matching the best code-based techniques.

### Shuffled: flat at small, up at medium, down at large

Shuffled barely moves at small (+0.005), improves at medium (+0.132), and
regresses at large (−0.027). The medium improvement is real but comes mostly
from a better-calibrated model overall rather than path features directly
helping distinguish good from shuffled code — shuffling tokens within a record
doesn't change `_file_path`.

The regression at large mirrors `imports_scope` (-0.042 shuffled at large): the
200-dimensional path vocabulary occupies capacity that would otherwise go to
learning intra-repo token-level style.

### Injected: strong gains at small and medium, flat at large

Injected at small (+0.262) and medium (+0.264) improve substantially. Unlike
imports, file paths have meaningful within-repo variance: `tests/test_model.py`
looks different from `pydantic/fields.py` even within the same repo. This
gives the path features some genuine intra-repo discriminative value — the model
can learn "test files vs source files have different hunk patterns". At large,
the gain disappears (−0.016), suggesting the benefit is only present when the
dataset is small enough for the path signal to remain informative.

### Comparison to imports_scope

| metric   | bucket | imports_scope | path_embed |
|:---------|:-------|:-------------|:-----------|
| cross    | small  | +0.295        | **+0.421** |
| cross    | medium | +0.293        | **+0.315** |
| cross    | large  | +0.118        | **+0.164** |
| shuffled | small  | −0.022        | **+0.005** |
| shuffled | large  | −0.042        | **−0.027** |
| injected | medium | +0.201        | **+0.264** |
| injected | large  | −0.039        | −0.016     |

Path embed dominates imports_scope on every metric. Both are repo-identity
signals, but path features have more within-repo variance (different files have
different paths) while imports are often identical across records in the same
repo. This extra within-repo variance is why injected improves more and shuffled
regresses less.

## Verdict

**Do not merge.** The cross-repo gains are the largest of any technique but
cross-repo AUC at 0.867 at small is almost trivially explained: the model has
learned "effect files are in `packages/*/ts`, pydantic files are in
`tests/*.py`" — which is correct but is repo fingerprinting, not style
assessment.

Shuffled regresses at large (−0.027) for the same reason as imports: path
features occupy capacity without contributing to intra-record style
discrimination.

For the argot use-case (does this commit look consistent with this repo's
style?), char n-grams (technique 4) remains the strictly better choice: it
improves every metric at every bucket size with no regressions.

**Future direction**: file path could be a highly effective lightweight
repo-routing signal — "which model to load" or "is this file even in scope?" —
separately from the style scoring model itself. A path-only classifier at
inference time could also serve as a fast pre-filter before running the full
JEPA model.
