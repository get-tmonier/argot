# Call-receiver scorer — parse-fragment artifact and fix

## Setup

> **Run:** `benchmarks/results/20260424T074736Z/` — 5-seed no-sample full
> bench, all 6 corpora, α=1.0 + `_has_root_error` content guard. This is
> the shipping configuration for era 6.

After four bench configurations for the call-receiver scorer failed the
pre-registered ship gates (presence-based k=1 / k=2, soft-penalty α=0.5 /
α=1.0), the prevailing agent narrative blamed "test-file vocabulary
absent from the attested set." Two evidence-doc passes repeated this
claim without verifying it against the raw bench JSONs.

The claim was wrong in a load-bearing way, and the real cause was
trivial to fix once the investigation actually looked at the false
positives.

## Why the "test files" hypothesis was wrong

The path-exclusion filter at
[`engine/argot/scoring/calibration/random_hunk_sampler.py:53–66`](../../../engine/argot/scoring/calibration/random_hunk_sampler.py)
already covers test content:

```python
for part in rel.parts[:-1]:
    if part in exclude_dirs or part.startswith("test") or part == "__tests__":
        return True
name = rel.name
if name.startswith("test_") or name == "conftest.py":
    return True
return ".test." in name or ".spec." in name
```

This is applied to control hunks at inference in
[`benchmarks/src/argot_bench/run.py:136`](../../../benchmarks/src/argot_bench/run.py)
before the scorer is invoked; any matching hunk short-circuits with
`reason="excluded_path"` and is excluded from the FP denominator.

Test hunks were never in the ~322 rich or ~367 ink false positives
produced at α=1.0. Verifying this took reading twenty lines of code.

## Phase 1: look at the actual FPs

A one-off investigation script (not committed) loaded
`benchmarks/results/20260424T072403Z/{rich,ink}.json`, filtered to
`flagged=True ∧ reason="call_receiver"`, and grouped by leading path
segment.

**rich (225 of 244 control FPs had `reason="call_receiver"`):**

| Path segment | Count | Examples |
|:---|---:|:---|
| `rich/`      |  225  | `rich/traceback.py`, `rich/table.py`, `rich/console.py`, `rich/panel.py` |

Zero FPs from tests, docs, examples, or data dirs. 100% from **core
library source files**.

Top unattested callees (re-derived via scorer replay on the same JSON):

```
theme                    23
locals_max_string        22
console                  21
title                    21
detect.height            21
locals_max_depth         18
cells                    18
width                    16
fonts.Defaults.padding   16
to                       16
```

These are not function names. They are **Google-style docstring parameter
descriptions**:

```python
def foo(x, y):
    """Render the panel.

    Args:
        theme (Theme): the color theme to apply.
        locals_max_string (int): truncate locals longer than this.
        console (Console | None): the target console.
        ...
    """
```

When the benchmark extracts `lines[hs:he]` from the file and passes that
fragment to `extract_callees`, the substring lands inside the triple-quoted
docstring without the opening `"""`. Tree-sitter-python, seeing
`theme (Theme): the color theme.`, cannot recognize the docstring form and
parses `theme (Theme)` as a `call` expression with callee `theme`. Because
no `theme(Theme)` call occurs in the real attested set, it is flagged as
unattested.

**ink (83 of 160 control FPs had `reason="call_receiver"`):**

| Path segment | Count | Examples |
|:---|---:|:---|
| `src/`       |   83  | `src/reconciler.ts`, `src/ink.tsx`, `src/render.ts` |

Top unattested callees:

```
removeChildFromContainer    18
commitUpdate                14
commitTextUpdate            12
prepareScopeUpdate           9
getInstanceFromScope         9
removeChild                  8
setCurrentUpdatePriority     8
```

These are **method-shorthand definitions** inside a React reconciler host
config:

```tsx
const reconciler = createReconciler({
  // ...
  commitTextUpdate(node, _oldText, newText) { node.nodeValue = newText; },
  commitUpdate(node, _updatePayload, ...) { ... },
  removeChildFromContainer(container, child) { ... },
});
```

When the hunk slice contains just the shorthand block without the
enclosing `createReconciler({ ... })`, tree-sitter-typescript parses
each method head `commitTextUpdate(node, _oldText, newText)` as a
`call_expression` followed by a block statement. Same failure mode as
rich's docstrings — the fragment lacks its syntactic container.

## The common shape

In both cases the extracted hunk does not parse as a standalone program.
Tree-sitter returns a tree with **root-level ERROR nodes**. Under those
ERROR nodes, the parser preserves typed children (`call`,
`call_expression`, `identifier`, `string`) heuristically, but the
**interpretation of those children is wrong** because the syntactic
context that would have reclassified them (triple-quote, object-literal
wrapper) is gone.

Quantified across the alpha=1.0 run:

| Corpus | `call_receiver` FPs | Root-level ERROR nodes | Fraction |
|:---|---:|---:|---:|
| rich | 225 | 202 | 89.8% |
| ink  |  83 |  71 | 85.5% |

This is not a scorer design problem. It is a tree-sitter-fragment
interaction that affects every stage of argot's pipeline in principle,
but only the call-receiver scorer surfaced it at a rate that breaks the
FP gate (because call-expressions are its entire input signal).

## The fix

Six lines in `benchmarks/src/argot_bench/call_receiver.py`:

```python
def _has_root_error(source: str, language: Language) -> bool:
    parser = _parsers[language]
    tree = parser.parse(source.encode("utf-8"))
    return any(child.type == "ERROR" for child in tree.root_node.children)

# inside _get_distinct_unattested
if _has_root_error(hunk_content, language):
    return []
```

Root-level ERROR ⇒ return zero unattested callees ⇒ zero soft penalty ⇒
Stage 1.5 silent ⇒ BPE decides as it did in era 5.

Why this does not hurt recall: **break fixtures are complete code
sections** (full function bodies, full class definitions pasted into
injection hosts). They parse cleanly. All 91 fixtures across all 6
corpora continue to flag as they did pre-fix. Confirmed by the shipping
run — 0/91 category regressions.

Why the guard is justified: the scorer's premise is "unattested callee
in this hunk." If the parser can't reliably identify callees in the
fragment, the scorer's premise does not hold and abstention is the
correct behavior. A subsequent stage (BPE) still sees the fragment and
can flag it independently.

## Projected vs observed impact

Replaying the α=1.0 JSON with the guard before running the real bench:

| Corpus | FP before fix | FP projected | FP observed (20260424T074736Z) |
|:---|---:|---:|---:|
| rich | 2.80% | 0.36% | 0.50% |
| ink  | 2.20% | 0.49% | 1.08% |

Observed FP is slightly higher than projected in both cases because the
shipping run re-sampled calibration hunks per seed and the α=1.0-no-fix
JSON was replayed against a fixed threshold. Both corpora still clear the
1.5% gate comfortably.

## Gate evaluation

| # | Gate | Threshold | Observed | Verdict |
|---|---|---|---|---|
| 1 | Average recall ≥ 80.0% | ≥ 80.0% | 80.8% | PASS |
| 2 | No corpus recall regresses > 2 pp | ≥ −2 pp | min +0.0 pp | PASS |
| 3 | All corpora FP ≤ 1.5% | ≤ 1.5% | max 1.08% (ink) | PASS |
| 4 | Category regressions from 100% | 0 / 91 | 0 / 91 | PASS |

All four gates clear. Era 6 ships.

## Interpretation

Three lessons worth keeping for future-era work:

1. **Agents with a confident narrative will repeat an unverified claim.**
   Two consecutive evidence docs attributed the FPs to test-file
   vocabulary. Neither verified against the raw data. One investigation
   phase (ten minutes, one Python script) disproved both by loading the
   JSON and grouping by path. Pre-register a "look at the data before
   theorizing" step for any regression that doesn't resolve after one
   pre-declared fallback.

2. **Tree-sitter on hunk fragments is not free.** The parser is
   error-tolerant by design and will emit a partial tree for any input,
   including half-docstrings and half-object-literals. "Partial" can
   mean "confidently wrong" in specific interactions. Era-4's
   `ImportGraphScorer` sidestepped this because imports only fire at
   syntactic positions that parse consistently. Era-6's call-receiver
   doesn't have that guarantee and needs the content guard.

3. **The post-hoc-tuning constraint can hide investigation steps.** The
   era-5 pre-registration discipline is correct and saved us from tuning
   α arbitrarily. But it also risks treating every gate failure as a
   design dead end, when the actual cause may be a plumbing bug the
   pre-registered configs can't isolate. The fix here was not a new
   scorer variant — it was a content guard applied inside the existing
   α=1.0 configuration. We ran four pre-declared variants before
   inspecting the JSON; in hindsight, a post-alpha-1.0 investigation
   phase should have been the pre-declared step, not another config.

The scorer itself was correct from the start — the signal it measures
(unattested call receivers) does catch the foreign-receiver breaks we
targeted. What needed fixing was the input contract: don't feed a
fragment to a parser and trust the output.

## Production port (PR 2)

### Port scope

Files changed:

**Created:**
- `engine/argot/scoring/scorers/call_receiver.py` — `extract_callees`, `_has_root_error`, `CallReceiverScorer` (with `alpha`/`cap` params replacing the research `k` param)
- `engine/argot/tests/test_call_receiver.py` — unit tests (28 tests: extractors, scorer fit, count_unattested, root-error guard, data-dominant skipping)

**Modified:**
- `engine/argot/scoring/adapters/language_adapter.py` — `extract_callees(source: str) -> list[str]` added to protocol
- `engine/argot/scoring/adapters/python_adapter.py` — `extract_callees` implemented (delegates to scorer module)
- `engine/argot/scoring/adapters/typescript.py` — same
- `engine/argot/scoring/scorers/sequential_import_bpe.py` — Stage 1.5 integration; `call_receiver_alpha=1.0`, `call_receiver_cap=5` constructor params; `"call_receiver"` added to `Reason` type
- `engine/argot/tests/test_sequential_import_bpe.py` — 5 precedence tests added (alpha=0 disables, alpha>0 builds scorer, import > call_receiver, no flag when attested, call_receiver tips threshold)
- `engine/argot/scoring/calibration/__init__.py` — writes `call_receiver_alpha`/`cap` to scorer-config.json
- `engine/argot/check.py` — reads alpha/cap from config; defaults to 1.0/5 for existing configs without these fields

**Deleted:**
- `benchmarks/src/argot_bench/call_receiver.py` — research module, superseded by engine implementation
- `benchmarks/tests/test_call_receiver.py` — bench-side tests, superseded by engine tests

**Bench adapter cleanup:**
- `benchmarks/src/argot_bench/score.py` — `BenchScorer` simplified to thin pass-through; alpha/cap routed into `SequentialImportBpeScorer`; old stub-based formula tests removed from `benchmarks/tests/test_score.py`

### Bench parity verification

Full 5-seed bench run (`uv run argot-bench --call-receiver-alpha 1.0`) vs era-6 baseline (`20260424T074736Z`):

| Corpus   | Target recall | Actual | Target FP | Actual | Drift |
|----------|-------------:|-------:|----------:|-------:|-------|
| fastapi  |  91.7%        | 91.7%  | 0.1%      | 0.1%   | 0.0 pp |
| rich     | 100.0%        | 100.0% | 0.5%      | 0.5%   | 0.0 pp |
| faker    | 100.0%        | 100.0% | 0.4%      | 0.4%   | 0.0 pp |
| hono     |  60.0%        | 60.0%  | 0.4%      | 0.4%   | 0.0 pp |
| ink      | 100.0%        | 100.0% | 1.1%      | 1.1%   | 0.0 pp |
| faker-js |  33.3%        | 33.3%  | 0.8%      | 0.8%   | 0.0 pp |

All 6 corpora match exactly — 0.0 pp drift on all numbers. The port reproduces the research results byte-for-byte.

### Observations from port

- **Parser reuse confirmed:** `_PY_PARSER` and `_TS_PARSER` imported directly from `filters.typicality` — no new instances created, the 30 GB per-hunk memory issue avoided.
- **API delta:** Research `CallReceiverScorer` took `k` (presence gate threshold); production takes `alpha` and `cap` (soft-penalty config). The `score_hunk` method is removed — only `count_unattested` is exposed, since threshold-comparison logic lives in `SequentialImportBpeScorer.score_hunk`.
- **Existing configs:** `check.py` defaults alpha=1.0, cap=5 for configs without those fields. Existing `.argot/scorer-config.json` files automatically enable Stage 1.5 on `argot check` after upgrading — intentional, matches the era-6 shipping config.
- **Zero numerical drift:** The production path computes identical decisions to the research wrapper path at alpha=1.0. The `_has_root_error` guard, the attested-set construction (using the same `exclude_data_dominant` filtered file list), and the soft-penalty formula all round-trip exactly.
