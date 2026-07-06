# Phase 1b — fastapi local dev loop (does argot help BEFORE CI?)

Setup: an agent with the argot-check skill implements a realistic "retry with
backoff" util in fastapi/, using `argot check` in its normal pre-commit loop.

## RESULT: yes — a clean catch → rework, before any CI

- Agent's honest first instinct: **`tenacity`** (the de-facto retry lib). Wrote
  `fastapi/_retry.py` wrapping tenacity. tenacity wasn't even installed — it wrote
  against the familiar lib first, as devs do.
- `argot inspect` → **Ready** (so the hit is trustworthy).
- `argot check` → exit 1, ONE hit:
  `foreign import · tenacity — 0 of 74 module specifiers; common here: fastapi
  (357×), pydantic (129×), typing (129×)`. severity **foreign**, score 1.0.
- The agent reworked to a stdlib `time.sleep` loop (same public surface, zero new
  deps). Re-check → **clean, exit 0**.
- Agent verdict: "argot did a reviewer's job pre-commit. FastAPI keeps a tiny
  runtime dep set; no lint rule catches 'don't add tenacity' — it would only have
  surfaced in human PR review. Not noise, not a false positive."

## GOOD (agent's words)
1. The one hit that mattered, nothing else — perfect signal-to-noise.
2. Evidence was actionable (why + what to use instead) → instant fix decision.
3. inspect-before-check ritual builds trust; skill orders it well.
4. Fast, ran on untracked files, clean JSON.
5. Correctly ignores tests/.

## PAIN / honest limits (the gold)
1. **The agent hit the documented limit naturally**: its stdlib rework uses a
   BLOCKING `time.sleep` in an async-first framework — a real design smell (should
   be async/anyio). argot passes it clean because every token is in-vocab; the
   *choice* is the issue. "A clean run is 'no foreign pattern', not 'idiomatic'."
   The skill warns this — but a newcomer could over-read green exit-0 as a
   blessing. → Validates our honest-limits messaging; good docs material.
2. **Skill gap**: the JSON field names you branch on (`reason_label`, `source`,
   `threshold` vs `score`) aren't documented in SKILL.md — inferred from output.
   → ACTIONABLE FIX.
3. **Confusing raw numbers**: for the untracked-file import hit, `score: 1.0` vs
   `threshold: 12.62` (the new-file BPE threshold, irrelevant to the categorical
   import signal). Severity label is right; the raw threshold display is
   misleading for import hits. → minor display inconsistency, note.
