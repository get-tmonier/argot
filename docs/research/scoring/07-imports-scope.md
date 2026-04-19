# Technique 5 — imports_scope (imports as a separate signal)

**Branch**: `research/imports-scope`  
**Status**: complete 2026-04-19  
**Baseline**: [`02-sizing-study.md`](02-sizing-study.md)

## Hypothesis

Import statements carry strong repo-identity signal: which packages a codebase
uses, which internal modules it pulls from, and whether it uses relative vs
absolute paths. Extracting these module strings as a separate TF-IDF feature
vector and concatenating it to the main context/hunk features should help the
model learn "this is an effect repo" vs "this is a pydantic repo" — lifting
cross-repo AUC.

## What changed

**`corpus.py`** gains `_extract_import_strings(tokens)`: walks the raw token
list (which still has `node_type` before `_load_records` slims it) and collects:
- **JS/TS**: `string_fragment` tokens within 4 tokens of a `from` keyword
  (e.g. `'../utils/merge.js'`, `'ky'`, `'react'`)
- **Python `from X.Y import Z`**: dotted identifiers between a `from` and the
  next `import` keyword (e.g. `click.testing`, `concurrent.futures`)
- **Python bare `import X`**: identifier after `import` where no `from` follows
  within 6 tokens (e.g. `sys`, `pytest`)

`_load_records` gains `imports_scope=False`: when True, extracts import strings
from raw tokens before slimming and stores them as `_imports` on each slim
record. `_imports` survives `shuffle_negatives` and `inject_foreign` because
both use `{**r, ...}` dict spread.

**`train.py`** gains `imports_scope=False` on `train_model`: when True, fits a
separate `TfidfVectorizer(max_features=500)` on `r["_imports"]` strings, then
concatenates the import features to both `ctx_x` and `hunk_x` before training.
The encoder's `input_dim` grows to `actual_word_dim + actual_import_dim`.

**`validate.py`** `score_records` checks `bundle.import_vectorizer`: if set,
extracts `r.get("_imports", "")` from records and concatenates import features
to ctx and hunk features before scoring.

`ModelBundle` gains `import_vectorizer: TfidfVectorizer | None = None`.

The CLI gains `--imports-scope` (flag). The justfile gains
`research-benchmark-imports-scope`.

## Results

Baseline is Phase 2 mean ± std (3 seeds). Variant: 3 seeds,
small/medium/large only.

### Shuffled AUC

| bucket | baseline       | variant        | Δ       |
|:-------|:---------------|:---------------|:--------|
| small  | 0.500 ± 0.000  | 0.478 ± 0.003  | **−0.022** |
| medium | 0.501 ± 0.000  | 0.577 ± 0.017  | +0.076  |
| large  | 0.637 ± 0.005  | 0.595 ± 0.003  | **−0.042** |

### Cross-repo AUC

| bucket | baseline       | variant        | Δ       |
|:-------|:---------------|:---------------|:--------|
| small  | 0.446 ± 0.023  | 0.741 ± 0.020  | **+0.295** |
| medium | 0.395 ± 0.004  | 0.688 ± 0.009  | **+0.293** |
| large  | 0.544 ± 0.012  | 0.662 ± 0.008  | **+0.118** |

### Injected AUC

| bucket | baseline       | variant        | Δ       |
|:-------|:---------------|:---------------|:--------|
| small  | 0.450 ± 0.017  | 0.623 ± 0.009  | +0.173  |
| medium | 0.398 ± 0.009  | 0.599 ± 0.014  | +0.201  |
| large  | 0.631 ± 0.008  | 0.592 ± 0.001  | **−0.039** |

## Interpretation

**Import features make the model a strong repo-identity detector, but at the
cost of intra-repo style discrimination.**

### Cross-repo: biggest gain in the experiment series

Cross-repo AUC jumps to 0.741 at small (+0.295), 0.688 at medium (+0.293),
and 0.662 at large (+0.118) — the largest cross-repo gains of any technique
tried. The reason is almost mechanical: the import vocabulary is nearly
deterministic per repo. Effect code imports from `@effect/*`; pydantic imports
from `pydantic.*`; ky imports from `ky`; click imports from `click`. Once the
model learns these associations, it scores foreign records high (wrong imports
for this repo) almost by definition.

### Shuffled: regression at small and large

Shuffled AUC drops at small (−0.022) and large (−0.042), and only improves at
medium (+0.076). Shuffling the hunk tokens within a record doesn't change the
`_imports` field — imports are extracted from the full record before slimming
and survive unchanged. So the import features add zero discriminative signal for
the shuffled task, but they occupy 500 of the total ~5,500 feature dimensions.
This capacity displacement hurts the model's ability to detect token-order
anomalies at small and large.

### Injected: mixed, regression at large

Injected AUC improves at small (+0.173) and medium (+0.201) but regresses at
large (−0.039). For injected records, `_imports` comes from the home record (via
`{**r, "hunk_tokens": foreign_hunk}`), so both ctx and hunk receive home-repo
import features even though the hunk is actually foreign. The model can't detect
the mismatch via imports — it sees "this is home context" on both sides. The
improvement at small/medium is likely a secondary effect of the model becoming
better-calibrated for those sizes, not a direct import signal.

### What the pattern reveals

Import features answer "which repo is this code from?" rather than "does this
hunk fit the style of its context?". The cross metric cares about the first
question; shuffled and injected care about the second. This technique improves
the wrong thing for the primary argot use-case: `argot check` is asked whether a
specific change looks consistent with *this* repo's style — not whether the
change looks like it came from a different repo.

## Verdict

**Do not merge.** The cross-repo gains are real and large, but they represent the
model learning repo identity (package usage) rather than style patterns. Shuffled
AUC regresses at two bucket sizes, injected regresses at large. The overall
quality picture is worse for the primary use-case.

If argot were exclusively used as a cross-repo similarity tool, imports would be
a strong addition. For its actual purpose (per-repo style checking), char n-grams
(technique 4) is a strictly better choice — it improves all metrics at all
buckets.

**Future direction**: imports could be useful as a lightweight repo fingerprint
for a separate "is this code in-scope for this repo?" pre-filter, separately from
the style scoring model.
