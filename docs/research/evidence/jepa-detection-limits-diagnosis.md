# JEPA detection limits: a vocabulary detector, not a semantic reasoner

## Setup

Phase 10 opened with a targeted re-read of the Phases 1–9 signal
sweep against the v2 fixture set (27 fixtures, 9 categories) to
characterise what JEPA was actually doing before swinging at a third
architecture. The predictor was trained on commit history for a
target repo and scored hunks by surprise: "what does a normal hunk
look like, given the context before it?" The diagnosis partitioned
the 9 categories into breaks JEPA caught and breaks it silently
missed, then named the axis that separated them.

## Results

JEPA is fundamentally a vocabulary-level + structural-sequence detector, not a semantic reasoner. It caught breaks at hunk OOV rate ≥ 8% against corpus vocabulary at delta ≥ 0.20:

| category | example | why it worked |
|:---|:---|:---|
| `routing` | Flask `@app.route(..., methods=[...])` in FastAPI | `jsonify`, `abort`, `app.run` zero-frequency |
| `framework_swap` | Django CBVs, aiohttp, tornado | entire class hierarchies absent |
| `serialization` | orjson at every call site | `orjson`, `OPT_INDENT_2` absent |
| `exception_handling` | `ValueError` instead of `HTTPException` | raise-position classes absent |

It silently failed on "correct vocabulary, wrong idiom" breaks:
`requests.get()` inside `async def` (requests appears 8× in corpus),
`threading.Thread` inside endpoints (token present, structural
problem invisible), marshmallow validators (`Schema`, `fields`,
`Validator` all corpus-present, only the module import carries
signal). Seven stages of hyperparameter tuning could not move delta
past ~0.09–0.14 on the v2 set.

## Interpretation

The partition by hunk OOV rate made the ceiling diagnosable, not
mysterious: any scorer that treats "anomalous against corpus
frequency" as a proxy for "non-idiomatic" inherits the same blind
spot as JEPA on subtle-idiom breaks. This reframed the next move —
before the third architecture swing, find out whether the hunk is
statistically different at the token level and on which axis.

---

*Source on tag `research/phase-14-pre-cleanup`:
`docs/research/scoring/signal/jepa_detection_limits.md`. Re-written
here for clarity, not copied.*
