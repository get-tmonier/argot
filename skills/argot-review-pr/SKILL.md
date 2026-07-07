---
name: argot-review-pr
description: Review a specific pull request (or diff range) against this repo's learned voice with argot, without checking it out — flag dependencies, APIs, and constructs foreign to how the repo is written. Advisory only, never blocks. Use when the user asks to "review PR #123 with argot", "check this PR for out-of-voice code", or "run argot on that branch/range". Distinct from argot-check (your local working changes) and argot-setup-ci (wiring the GitHub Action).
---

# argot-review-pr

Score a pull request against the repo's **local** fitted voice and report
anything foreign to it — **as advice, never as a gate**. argot is statistical;
false positives are expected. The human decides what to do.

`argot review` scores the PR's diff without checking it out, using the model
already fitted in `.argot/`, so it's fast and leaves the working tree untouched.

## Preconditions

1. `argot --version` — if missing, tell the user how to install it (see
   <https://argot.tmonier.com/docs/getting-started/>) and stop.
2. The repo must be fitted locally: `.argot/scorer-config.json` exists. If not,
   run the **argot-setup** skill (or `argot init`) first, then continue.
3. For a PR by number/URL, the `gh` CLI must be authenticated (argot fetches the
   PR diff through it). A `base..head` range or commit sha needs no network.

## Run it

The target is a PR URL, `#number` / `number`, a `base..head` range, or a sha:

```
argot review 123 --format json
argot review https://github.com/org/repo/pull/123 --format json
argot review origin/main..my-branch --format json
```

Exit codes: `0` clean · `1` hits found · `2` setup/usage error. **Treat `1` as
"there is something to look at," not as a failure.**

Each hit in the JSON `hits` array carries `severity`
(`foreign` / `suspicious` / `unusual`), `reason_label`, `evidence` (the lines to
show — the foreign symbol and what the repo uses instead), `hash`, and
`path` / `line_start` / `line_end`. Read `severity`, not the raw `score` /
`threshold` (those sit on different scales per signal).

## Gauge trust first

Run `argot inspect` and read the verdict. If it's **Marginal** or **Not
recommended**, the model isn't well-calibrated on this repo — down-weight every
hit accordingly and say so.

## What a hit means — and what a clean run doesn't

argot reliably flags a **novel pattern** foreign to this repo: a dependency it
has never imported, an API it never calls, or a whole paradigm it never writes.
Trust a `foreign` hit here. It does **not** reliably catch *in-vocabulary*
breaks — where every token is already in the repo and only the choice is wrong.
So a clean review means "no foreign pattern found," not "this PR is idiomatic."

## Decision tree (never block)

- **`foreign`** — high-confidence anomaly. Surface it, read the evidence line,
  and ask whether it matches how the repo already does this. If the choice is
  deliberate, the reviewer can record it: `argot mute <hash> --reason "…"` (a
  committed `[[mute]]` in `argot.toml`).
- **`suspicious`** — mention as worth a glance; show the evidence.
- **`unusual`** — usually fine; raise only if relevant.

## Hard rules

- **Never block, fail, or reject the PR** because argot fired. No hit is a merge
  gate — argot informs the review, the human decides.
- **Never rewrite the PR author's code** to silence a hit. Suggest; don't enforce.
- **Never mute on someone's behalf** without a real reason they'd endorse.
- False positives are normal. If a hit is fine, offer the `argot mute` command
  with a reason so it doesn't come back.
