---
title: What it catches
description: The five axes argot flags — a foreign dependency/API/paradigm the repo has never used, a redundant function it already has, misplaced code, an import that breaks the repo's layering, and a test gamed to green a failing suite — plus an honest account of the in-vocabulary breaks it still won't gate on.
group: Guide
order: 8
---

argot catches code that is *technically fine but doesn't fit this project* — valid, typed, and
lint-clean, but not how this codebase writes things. It works on **five axes**:

- **Foreign** — a dependency, API, or whole paradigm the repo has never reached for. The base voice
  model (statistical, no neural net; rules `foreign-import`, `unfamiliar-callee`, `rare-tokens`);
  the class an AI agent trips most, and the one the published numbers gate on.
- **Redundant** — a new function that reinvents one the repo already has. The semantic layer finds
  the original and shows you where it lives (rule `redundant`).
- **Misplaced** — the right code, filed in the wrong package. Also the semantic layer (rule
  `misplaced`).
- **Layering** — an internal import that reverses the repo's own layer direction. The architecture
  detector (rule `layering`).
- **Integrity** — a test quietly deleted, disabled, or weakened alongside the production change it
  covers. The integrity detector (rules `test-deleted`, `test-disabled`, `test-weakened`) — the
  complement of the other four: they catch foul play with *code*, this one catches foul play with
  *tests*.

Every rule defaults to severity `error` — a finding fails `argot check` — except `test-weakened`,
which ships `warn` (reported, never fails the check); every rule can be reconfigured per repo or
per run. See [Configure](/docs/configure/#rules--rule-severities).

Everything below is a real result from the shipped binary (`argot check` on a planted hunk, fit on
the repo's own history). Where argot flags a line, the transcript is quoted verbatim. Where it
doesn't, that's said plainly.

## Foreign — a pattern the repo has never used

This is the base voice model — statistical, no neural net. Across the whole fixture set in 11
languages, when the foreign symbol is visible in the code — an explicit import, a fully-qualified
call, a distinct API name — argot catches **604 of 618 (98%)** on the honest, leak-free bench.
Three shapes of foreign code:

### 1. A foreign dependency

```python
# an agent adds an outbound call — with the HTTP client it reflexively reaches for
import requests                          # this codebase standardises on httpx
```

```text
! foreign · requests
  ↳ 0 of 74 module specifiers in this repo
  common here: fastapi (357×), pydantic (129×), typing (129×) …
```

`requests` is a fine library — it's just *foreign to this repo*. The **import stage** flags the new
top-level module directly. No linter flags it without a hand-maintained banned-imports list; argot
learned "this repo's 74 imports are fastapi, pydantic, starlette… never requests" from git history and
shows the receipts.

### 2. A foreign API

```python
# a call into a data layer the repo standardises away from
from pymongo import MongoClient
_audit = MongoClient("mongodb://localhost:27017")["app"]["audit"]

def record_dependency_call(name: str, resolved: bool) -> None:
    _audit.insert_one({"dependency": name, "resolved": resolved})
```

```text
? suspicious · unfamiliar-callee
  ↳ _audit.insert_one — 0 of 927 callees in this cluster
```

The tell here isn't only the import — it's the **call**. Even where the winning signal is the
call-receiver stage flagging `_audit.insert_one` as a callee the corpus never attests, argot is reading
*how you use* the library, not just what you imported. A dependency-allowlist watches `package.json`;
it cannot watch method calls. argot does.

### 3. A foreign paradigm  — *the one that sells it*

An agent writes a handler as a **Django class-based view** in a repo that is entirely FastAPI:

```python
class ReceiptView(View):                 # FastAPI uses function endpoints + Depends()
    def get(self, request, user_id):
        receipt = self.repo.find(user_id)
        if receipt is None:
            return HttpResponseNotFound()
        return JsonResponse(receipt.to_dict())
```

```text
! foreign · unfamiliar-callee   [score 11.05]
  ↳ JsonResponse, HttpResponseNotFound — 0 of 927 callees in this cluster
```

Every line is valid Python. mypy is happy. ruff is happy. There is no single "bad import" to ban — it's
the **whole paradigm** that's foreign: `View` subclasses, `JsonResponse`, `HttpResponseNotFound`, the
request-first signature. argot flags it because that entire vocabulary of callees is absent from a
codebase built on typed function endpoints. **No linter, type checker, or dependency policy can encode
"we don't write Django-style views here" — argot learns it.** This is the gap between *valid* and
*ours*, and it is the class the base voice model is built to close.

## Redundant — a function you already have

The **semantic layer** embeds every function at fit and indexes it. At check, a *new* function that
duplicates one already in the repo is flagged — the nearest cross-file neighbour, with a similarity
margin:

```text
.  already implemented here (redundant)
   ↳ duplicates slugify (src/utils/text.py:1) — similarity 0.86
```

Real repos hold real duplication, and sometimes a second `slugify` is a deliberate call — argot
shows you the original and lets you judge. `redundant` findings are pinned to the `unusual`
confidence tier (the evidence is a similarity lookup, not a hard fact), and like every rule the
severity is yours: it fails the check by default, or one config line makes it report-only
(`redundant = "warn"` in `argot.toml`'s `[rules]`).

## Misplaced — the right code, wrong place

Also the semantic layer — a function whose nearest semantic neighbours concentrate in a *different*
package or area than the one it was filed under:

```text
.  unusual location (misplaced)
   ↳ looks like core/downloader code filed under commands/
```

Same posture — a nudge to check whether a helper landed in the wrong module, pinned to the
`unusual` confidence tier and configurable per rule (`misplaced = "warn"`, or `semantic = "off"` to
turn the whole group off). Both `redundant` and `misplaced` come from a per-repo code-embedding
index (`jina-code`, ~100 MB, local, CPU-first — no cloud, no LLM). See
[How it works](/docs/how-it-works/) for the mechanics.

## Layering — an import that breaks your architecture

The **architecture detector**. At fit, argot builds a module-dependency graph of the repo
(`.argot/layering.json`); at check, an added internal import that *reverses* an established layer
direction — a low-level module suddenly importing from the layer above it, or an edge out of a
module the graph knows as a (near-)sink — is flagged:

```text
.  crosses a module boundary (layering)
   ↳ core/parser now imports cli/commands — the repo's edges run the other way
```

This is the structural cousin of the foreign axis: every token can be repo-familiar, but the
*edge* is one the codebase has never drawn. On the benchmark — 23 corpora across all 11 supported
languages — the detector caught **244 of 252 (96.8%)** planted layering violations with **0 of 140**
false positives on control edits, and a worst-case over-fire of 2.7%. Findings are pinned to the
`unusual` confidence tier and fail the check by default (`layering = "warn"` or `"off"` to
downgrade). The fit-time import resolver covers Python in v1; the graph and benchmark methodology
are language-agnostic.

## Integrity — a test gamed to green a failing suite

The **integrity detector** — the complement of the other four. Where `foreign-import`,
`redundant`, `misplaced`, and `layering` catch foul play with *code*, this one catches foul play
with *tests*: an AI agent asked to make a failing suite pass that quietly deletes, skips, or
weakens the test instead of fixing the bug.

At fit, a mini-replay of the repo's own accepted history learns which test-editing patterns are
*normal* here — moved/renamed tests, assertions extracted to helpers, tests retired with their
feature, bulk migrations — and stores the per-repo gates in `.argot/integrity.json`. At check, any
changeset that touches production source *and* weakens, disables, or removes a test is flagged;
tests-only commits (suite curation, refactors, renames with no production change) are excused by
design:

```text
!  test disabled (test-disabled)
   ↳ test `test_parse` disabled — skip/ignore marker added; this change also modifies parser.py
```

Three rules: **`test-deleted`** — a test removed while the production code it exercised still
exists; **`test-disabled`** — a skip/ignore marker added, or a test gutted to a vacuous pass;
**`test-weakened`** — assertions removed, tautologized, or loosened, or an expected literal
retargeted. All three are pinned to the `suspicious` confidence tier. `test-deleted` and
`test-disabled` default to `error`; **`test-weakened` ships `warn`** — reported on every run,
never fails the check on its own, downgradeable or upgradeable per repo.

On the benchmark — 23 corpora across 12 languages — the detector caught **155 of 164 (94.5%)**
authored gaming tactics, with **0 of 106** legitimate-refactor controls (moves, renames, deletions
alongside a removed feature, genuine strengthening) fired. Replayed against 5,330 real accepted
test-touching commits *outside* the fit's calibration window, only **1.13%** were flagged at
gating (error) severity — the honest cost of running this against real history, not a fixture
rate. The hardest tactic is expected-value retargeting (16/21, 76%): a bare literal retarget is
statically indistinguishable from a healthy TDD update, so it only fires in repos whose accepted
history shows zero isolated literal flips — elsewhere the per-repo gate keeps it silent by design,
a documented limit, not a bug. Full numbers, per-tactic breakdown, and the FP-hardening journey:
[`docs/research/evidence/test-integrity-capstone.md`](https://github.com/get-tmonier/argot/blob/main/docs/research/evidence/test-integrity-capstone.md).

A clean integrity check means **no known gaming pattern fired** — not that the tests are good, and
not that every test is trustworthy. A fraudulent test that asserts the wrong behaviour from the
start needs an oracle, not a diff, and is out of scope by design.

## What argot does *not* reliably catch

Honesty is a feature. When a break reuses **only vocabulary the repo already has** — every token,
callee, and import is corpus-present, and the mistake is a *choice among familiar things* — argot
usually **does not** flag it, and the published numbers deliberately **do not gate on it**.

### Wrong exception type — *usually not caught*

```python
@router.get("/{user_id}", response_model=UserResponse)
async def get_user(user_id: int, db=Depends(get_db)) -> UserResponse:
    user = db.get(user_id)
    if user is None:
        raise ValueError(f"User {user_id} not found")   # repo always raises HTTPException — argot stays silent
    return user
```

argot does **not** flag this. `ValueError` and `HTTPException` are *both* in the repo's vocabulary; only the choice is wrong.
The semantic layer added real code understanding — it's what powers the reinvention and placement
checks above — but separating "wrong choice" from a legitimate `ValueError` elsewhere is a
finer-grained call than a nearest-neighbour lookup makes, and forcing the base model to fire on it
drives false alarms on the repo's own code (the recovery investigation measured **+1 recall for +45
false positives**, blowing the ≤1.17% budget). So argot leaves it — a **line it won't cross**. Same story for a **manual `if status_code >= 400` instead of `raise_for_status()`**,
or a **sync `def` endpoint** in an async repo: structurally non-idiomatic, but built from entirely
attested tokens — and verified clean on the bench.

This is a deliberate scope line, recorded in
[`docs/research/evidence/issue92-investigation-capstone.md`](https://github.com/get-tmonier/argot/blob/main/docs/research/evidence/issue92-investigation-capstone.md):
argot gates on the danger an LLM actually poses — dragging in a whole foreign pattern — not on subtle
misuse of your own vocabulary.

## The one-line summary

| argot flags | argot won't gate on |
|---|---|
| A dependency the repo has never imported *(`foreign-import`)* | A wrong *value* on an attested construct |
| A call into a library the repo standardises away from *(`unfamiliar-callee`)* | A wrong exception type where both are attested |
| A whole foreign paradigm — Django view, Flask route, manual validation *(`rare-tokens` / `unfamiliar-callee`)* | A structural shape built from only-familiar tokens |
| A new function that reinvents one you already have *(`redundant`)* | — |
| The right code filed in the wrong package *(`misplaced`)* | — |
| An internal import that reverses your layering *(`layering`)* | — |
| A test deleted, disabled, or weakened alongside a prod change *(`test-deleted` / `test-disabled` / `test-weakened`)* | A fraudulent test that asserts the wrong behaviour from the start |

Treat a hit as a prompt to look, never a verdict. **Foreign** catches are reliable — 98% when the
symbol is visible in the change. **Redundant** and **misplaced** surface the nearest existing code
and let you judge; **layering** shows the edge that runs against the graph. **Integrity** catches
94.1% of authored gaming tactics at a 1.12% cost on real accepted history, and reports
`test-weakened` findings without failing the check by default. Every rule is configurable
(`error` / `warn` / `off` — see [Configure](/docs/configure/#rules--rule-severities)). And there's
a **line it won't cross** — a wrong choice built entirely from vocabulary you already have; argot
won't gate on that, and says so.

See these catches on real code: [Caught in the wild](/caught-in-the-wild) collects verified findings
from running argot over 33 open-source repositories' git history — each one adversarially attacked by
a reviewer briefed to disprove it, and every one survived.
