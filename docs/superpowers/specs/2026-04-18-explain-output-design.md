# explain output redesign

**Date:** 2026-04-18
**Status:** approved

## Problem

`argot explain` produces output that feels disconnected from `argot check`. The violations
flagged by `check` (with tag, score, file, line) don't map visibly to the explanations
produced by `explain`. Users have to mentally join two different output formats. Additionally,
Claude calls currently fire and render as lines stream in from the engine, producing
non-deterministic ordering when multiple violations are present.

## Goal

Make `explain` output feel like a natural continuation of `check` — each section header
mirrors the check row format (file:line, tag, score, ref), and sections render in a stable,
ordered sequence.

## Output format

### Normal run (N violations)

```
argot · <name> · threshold <t> · model <model>
<N> violation(s) above threshold — explaining...

── [1/N] <file>:<line>  <tag>  <score>  <ref> ──────────────────────────────────

  <summary sentence(s)>

  • <concrete issue 1>
  • <concrete issue 2>
  • ...

── [2/N] <file>:<line>  <tag>  <score>  <ref> ──────────────────────────────────

  ...
```

- Section separator is an 80-char line: `──` prefix, violation info embedded left-to-right,
  `──` dashes pad the right side to fill width.
- `tag` is one of `unusual` / `suspicious` / `foreign`, matching `check` thresholds:
  - `score > threshold` and `score <= threshold + 0.3` → `unusual`
  - `score > threshold + 0.3` and `score <= threshold + 0.6` → `suspicious`
  - `score > threshold + 0.6` → `foreign`
- `score` printed to 4 decimal places.
- `ref` is `workdir` for uncommitted changes or the git ref string for committed ranges.
- Body: one summary paragraph (1–3 sentences), then bullet points for concrete issues.

### Zero violations

```
argot · <name> · threshold <t> · model <model>
No violations above threshold — nothing to explain.
```

## Rendering strategy

**Buffer-and-render:** collect all engine records and launch all Claude API calls in parallel
as records arrive. When the engine process closes, `Promise.all` the Claude results and render
everything in arrival order. A spinner runs during the wait.

Rationale: the engine (local model inference) finishes in milliseconds. Claude calls dominate
latency and run in parallel regardless of strategy. Buffering costs nothing measurable and
guarantees ordered, non-interleaved output.

## Changes required

### `engine/argot/explain.py`

Add a `tag` field to each emitted JSON record. Computed identically to `check.py`:

```python
def _score_to_tag(score: float, threshold: float) -> str:
    if score <= threshold + 0.3:
        return "unusual"
    elif score <= threshold + 0.6:
        return "suspicious"
    else:
        return "foreign"
```

Include `"tag": _score_to_tag(score, args.threshold)` in the `json.dumps` payload.

### `cli/src/modules/explain/infrastructure/adapters/out/bun-explainer.adapter.ts`

1. Add `tag: Schema.String` to `EngineRecord`.
2. Change rendering strategy:
   - On each `line` event: parse record, launch `Effect.runPromise(callClaude(...))`, push
     `{record, promise}` to a buffer array.
   - On `close` event: `Promise.all` the promises, then render in order:
     - Print count line: `\n<N> violation(s) above threshold — explaining...`
     - For each result: print 80-char separator with embedded info, blank line, indented
       summary, blank line, indented bullets, blank line.
3. Zero-violation case: print `No violations above threshold — nothing to explain.`

### `cli/src/shell/infrastructure/adapters/in/commands/explain.command.ts`

Update header line to include threshold:

```
argot · <name> (<gitRoot>) · threshold <t> · model <model>
```

### Tests

- Update `test_explain_smoke.py` to assert the emitted JSON record includes a `tag` field
  with value `unusual`, `suspicious`, or `foreign`.
- Unit test the tag computation function directly.
