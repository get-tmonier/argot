---
title: Real-world scenarios
description: argot dogfooded end-to-end on real repositories — setup, the local dev loop, muting, and the CI pull-request flow — with the actual transcripts, good and bad.
group: Guide
order: 10
---

Everything here is a real run of the shipped binary on real repositories —
[FastAPI](https://github.com/fastapi/fastapi),
[Saleor](https://github.com/saleor/saleor), and a
[Hono](https://github.com/honojs/hono) fork. The transcripts are quoted as they
came out, the misses included. If argot passed something it shouldn't have, you'll
find it said so.

## 1. Setup — one command, and one judgment call

`argot init` fits the voice model and gives a health verdict in seconds:

```text
$ argot init                              # Saleor — 4,284 Python files
  python: 4284 files (100%) · 1139 included · excluded: 3135 path, 10 data-dominant
Verdict: Ready                            # ~14s
```

On a real Django app this is **clean out of the box**: Saleor's **1,432 migration
files** and its tests are excluded automatically — `argot init --suggest` finds
nothing to add, because there's nothing to add.

The one judgment call is **framework repos with a large examples tree**. On
FastAPI, the authored library (`fastapi/`) is 48 files, but `docs_src/` — the
tutorial examples — is 454. Left in, the learned voice is ~90% example code:

```text
$ argot init                              # FastAPI, default
  python: 1119 files · 496 included       # ← 90% is docs_src tutorial code

$ echo "docs_src/" > .argotignore && argot init
  python: 1119 files · 48 included        # ← now it's the library's own voice
```

`--suggest` can't make this call for you — example code isn't *generated*, it's a
*semantic* question ("is our tutorial code part of our voice?"). A library
contributor excludes it; an app author building *on* the framework keeps it. This
is exactly the [`.argotignore`](/docs/configure/) moment an agent or human owns.

## 2. The local dev loop — argot as a reviewer, before CI

This is the scenario that matters most: does argot help you write in-voice code
*before* a commit ever happens? We gave an agent a real task on FastAPI — *"add a
small utility that retries a callable with exponential backoff"* — and the
[argot-check skill](/docs/agents/), and let it work naturally.

Its honest first instinct was **`tenacity`**, the de-facto Python retry library:

```python
# fastapi/_retry.py  — the agent's first draft
from tenacity import retry, wait_exponential, stop_after_attempt
```

Before committing, it ran `argot check` — and got one hit:

```text
! fastapi/_retry.py:1-33   foreign   · foreign import
  ↳ tenacity — 0 of 74 module specifiers in repo
    common here: fastapi (357×), pydantic (129×), typing (129×)
```

FastAPI keeps a deliberately tiny runtime dependency set — slipping `tenacity` in
as a hard dependency is the kind of thing a maintainer rejects on sight, and **no
lint rule catches it**. The agent reworked to a stdlib loop (same public surface,
zero new dependencies); the re-check was clean. In its words:

> *argot did a reviewer's job pre-commit. It would otherwise have surfaced only in
> human PR review. Not noise, not a false positive.*

### …and the honest limit, in the same run

The stdlib rework used a blocking `time.sleep` in an **async-first** framework — a
real design smell (it should probably be async). argot passed it **clean**,
because every token is already in FastAPI's vocabulary; only the *choice* is off.
That's the documented boundary, live:

> **A clean `argot check` means "no foreign pattern found" — not "this is
> idiomatic."** argot catches the foreign dependency an agent drags in; it does
> not catch a sync-in-an-async design decision. See
> [what it catches](/docs/what-it-catches/).

## 3. Intentional foreign code — accept it, with a trail

Sometimes the foreign thing is a real decision. Mute it by hash with a reason, and
it stops flagging — with an audit trail:

```text
$ argot check --staged
! saleor/…/client.py   foreign · tenacity [cbc8047c9ecc]

$ argot mute cbc8047c9ecc --reason "RFC-42: tenacity is our chosen retry library"
Muted [cbc8047c9ecc] — RFC-42: tenacity is our chosen retry library
# → the tenacity hit is now an accepted decision, and no longer flagged
```

The reason lands in `.argot/suppressions.yaml` and `argot list-mutes` — the next
reviewer sees *why*, not just that it was silenced.

## 4. Prevention — before a line is written

Detection is reactive. For prevention, hand the agent the repo's voice up front:

- **`argot describe-voice`** writes a summary of the repo's idioms and a
  "red flags" line — *what* argot will flag — as agent context.
- **`argot mcp`** exposes `voice_context` over MCP, feeding your editor agent the
  repo's established patterns before it generates code. See [Agents](/docs/agents/).

## 5. The CI pull-request flow — flag → fix → green

On a Hono fork, an agent opened a PR adding a receipts endpoint written
**Express-style** (`Router`, `req`/`res`) in an all-Hono codebase. The Action
posted a **non-blocking** voice-score comment:

```text
🎙️ argot voice check — 83% in-voice · 🔴 foreign · express
   src/helper/receipts/index.ts — advisory, not a merge gate
```

A follow-up commit rewrote it in Hono style (`new Hono()`, `c.json`). The **same
sticky comment updated in place** to green:

```text
🎙️ argot voice check — 100% in-voice ✅
   this diff sounds like the rest of the repo
```

> A pull request is scored on its **net diff** — the same thing a reviewer reads
> in the Files tab — so a fix commit clears an earlier commit's flag. The card
> never blocks the merge; the reviewer has the last word. See [CI](/docs/ci/).

## What the dogfood showed

- **It catches what it says it does.** A foreign dependency (`tenacity`, `aiohttp`,
  `express`) is flagged fast, with evidence specific enough that the fix is obvious
  — before CI, before human review. Signal-to-noise was high: one foreign import,
  one hit, no chaff.
- **It doesn't cry wolf.** `httpx` in FastAPI scored clean — it's a library the
  repo actually uses. argot flags *foreign*, not *unfamiliar-to-you*.
- **It's honest about the rest.** In-vocabulary design smells (sync in an async
  repo) pass clean. That's a documented scope line, not a hidden failure — treat a
  green check as "no foreign pattern," never as a blessing.
