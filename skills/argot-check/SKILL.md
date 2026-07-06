---
name: argot-check
description: Score your working changes with argot — flag code foreign to this repo's own patterns (unfamiliar dependencies, APIs, constructs) before committing. Advisory only, never blocks. Use after generating or editing code, before a commit, or when the user asks "check my changes with argot", "is this in-voice", or "does this match how we write code here".
---

# argot-check

Run `argot` on the current changes and report anything foreign to the repo's
learned voice — **as advice, never as a gate**. argot is statistical; false
positives are expected. The human decides what to do.

## Preconditions

1. `argot --version` — if missing, tell the user how to install it (see
   <https://argot.tmonier.com/docs/getting-started/>) and stop.
2. The repo must be fitted: `.argot/scorer-config.json` exists. If not, run the
   **argot-setup** skill (or `argot init`) first, then continue.

## Run it

Score the changes you care about, as JSON:

```
argot check --format json            # working-tree changes
argot check --staged --format json   # what's about to be committed
```

Exit codes: `0` clean · `1` hits found · `2` setup/usage error. **Treat `1` as
"there is something to look at," not as a failure.**

Each hit in the JSON `hits` array carries:

| Field | Use |
|---|---|
| `severity` | `foreign` / `suspicious` / `unusual` — branch on this (see the decision tree). |
| `reason_label` | Human label of the signal: `foreign import`, `unfamiliar callee`, `rare token sequence`. |
| `evidence` | The lines to show the user — names the foreign symbol and what the repo uses instead. |
| `hash` | Stable id for `argot mute <hash>`. |
| `path`, `line_start`, `line_end` | Where it is. |
| `source` | `workdir` / `staged` / `untracked` / a commit SHA — where the change came from. |
| `score`, `threshold` | Raw internals. **Read `severity`, not these** — they sit on different scales per signal (a foreign import scores 1.0 against its own bar of 1.0, unrelated to the BPE `threshold` shown), so comparing them directly is meaningless. |

## Gauge trust first

Run `argot inspect` and read the verdict. If it's **Marginal** or **Not
recommended**, the model isn't well-calibrated on this repo — down-weight every
hit accordingly and say so.

## What a hit means — and what a clean run doesn't

argot reliably flags one thing: a **novel pattern** foreign to this repo — a
dependency it has never imported, an API it never calls, or a whole paradigm
(a Django-style view in a FastAPI repo, a different HTTP client, hand-rolled
validation) it never writes. When the foreign symbol is in the change, it catches
~99% of these. Trust a `foreign` hit here.

It does **not** reliably catch *in-vocabulary* breaks — where every token is
already in the repo and only the choice is wrong (a bare `ValueError` where the
repo raises `HTTPException`; a manual status check instead of `raise_for_status()`).
So **a clean run means "no foreign pattern found," not "this matches every
convention."** Don't present a clean argot result as a guarantee the code is
idiomatic — it's silent on the subtle stuff by design.

## Decision tree (never block)

For each hit in the JSON (`severity`, `reason`, `evidence`, `hash`):

- **`foreign`** — high-confidence anomaly (a dependency/API the repo has never
  used). Surface it clearly. Read the evidence line — it shows the surprising
  identifier and what the repo uses instead. Ask: *does this match how the repo
  already does this?*
  - If a well-established in-voice option exists and the user is open to it,
    suggest switching.
  - If the choice is deliberate, tell the user they can record it:
    `argot mute <hash> --reason "…"`.
- **`suspicious`** — mention it as worth a glance; show the evidence.
- **`unusual`** — usually fine; raise only if directly relevant to what the user
  is doing.

## Report format

Give a short, calm summary. For example:

```
argot: 1 foreign · 1 suspicious in your changes (advisory — argot is statistical)

! src/http.ts:42   axios — 0 of 47 imports in this repo use it
                   the repo reaches for: react (320×), express (88×), pg (47×)
                   → intentional? record it: argot mute a1b2c3d4 --reason "…"
? src/api.ts:88    unusual token sequence — glance at the evidence
```

## Hard rules

- **Never block, fail, or refuse to proceed** because argot fired. No hit is a
  merge/commit gate.
- **Never rewrite the user's code** just to silence a hit. Suggest; don't
  enforce.
- **Never mute on the user's behalf** without a real reason they'd endorse.
  Muting is a human decision; offer the exact command instead.
- False positives are normal. If the user says a hit is fine, that's the end of
  it — offer to mute it with their reason so it doesn't come back.
