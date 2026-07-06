---
title: What it catches
description: The dependable catches — a foreign dependency, API, or whole paradigm the repo has never used — and an honest account of the in-vocabulary breaks it does not reliably catch.
group: Guide
order: 7
---

argot catches code that is *technically fine but foreign to this project* — valid, typed, and
lint-clean, but not how this codebase writes things. It is built for one shape above all: a **novel
pattern**, a dependency or API or paradigm the repo has never reached for. That is the class an AI
agent trips most, and the class the published numbers gate on.

Everything below is a real result from the shipped binary on the FastAPI benchmark (`argot check` on a
planted hunk, fit on the repo's own history). Where argot flags a line, the transcript is quoted
verbatim. Where it doesn't, that's said plainly.

## The dependable catches

<!-- TODO(js-numbers): the fixture total and the "N of M" catch fraction below still show the
     pre-JavaScript run; refresh them once the JS re-bench dashboard lands. -->
Across the whole fixture set in 11 languages, when the foreign symbol is visible in the code — an
explicit import, a fully-qualified call, a distinct API name — argot catches **~99%** on the honest,
leak-free bench. Three shapes, strongest first.

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
? suspicious · unfamiliar callee (call_receiver)
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
! foreign · unfamiliar callee (call_receiver)   [score 11.05]
  ↳ JsonResponse, HttpResponseNotFound — 0 of 927 callees in this cluster
```

Every line is valid Python. mypy is happy. ruff is happy. There is no single "bad import" to ban — it's
the **whole paradigm** that's foreign: `View` subclasses, `JsonResponse`, `HttpResponseNotFound`, the
request-first signature. argot flags it because that entire vocabulary of callees is absent from a
codebase built on typed function endpoints. **No linter, type checker, or dependency policy can encode
"we don't write Django-style views here" — argot learns it.** This is the gap between *valid* and
*ours*, and it is the class argot is built to close.

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
Separating "wrong choice" from a legitimate `ValueError` elsewhere needs semantic reasoning a no-model,
no-runtime binary doesn't have — and forcing it drives false alarms on the repo's own code (the
recovery investigation measured **+1 recall for +45 false positives**, blowing the ≤0.98% budget). So
argot leaves it. Same story for a **manual `if status_code >= 400` instead of `raise_for_status()`**,
or a **sync `def` endpoint** in an async repo: structurally non-idiomatic, but built from entirely
attested tokens — and verified clean on the bench.

This is a deliberate scope line, recorded in
[`docs/research/evidence/issue92-investigation-capstone.md`](https://github.com/get-tmonier/argot/blob/main/docs/research/evidence/issue92-investigation-capstone.md):
argot gates on the danger an LLM actually poses — dragging in a whole foreign pattern — not on subtle
misuse of your own vocabulary.

## The one-line summary

| argot flags | argot usually does **not** flag |
|---|---|
| A dependency the repo has never imported | A wrong *value* on an attested construct |
| A call into a library the repo standardises away from | A wrong exception type where both are attested |
| A whole foreign paradigm (Django view, Flask route, manual validation) | A structural shape built from only-familiar tokens |
| **Reliable — 99% when the foreign symbol is visible** | **Secondary — surfaced sometimes, never gated** |

Treat a hit as a prompt to look, never a verdict. The thread through everything argot *does* catch:
**the foreign symbol is right there in the change** — a new import, an unattested callee, a paradigm
the repo has never spoken.
