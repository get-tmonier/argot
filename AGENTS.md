# Working with argot

[argot](https://argot.tmonier.com) is a guardrail that flags code **foreign to a
repo's own patterns** — the dependencies, APIs, and constructs it learns a
codebase has never used, from its git history. It answers *"is this how we write
things here?"*, not *"is this valid?"*. This file tells a coding agent how to use
it well. (Developing argot itself? See `CLAUDE.md`.)

## The contract: advisory, never blocking

argot is a **statistical** linter. It has false positives, and its confidence
depends on the repo. **A hit is a prompt to think, never a gate.**

- Never block a commit, fail a task, or refuse to proceed because argot fired.
- Never auto-"fix" a hit by rewriting code the human didn't ask you to change.
- The human always has the last word. Your job is to *surface* divergences and
  *record decisions*, not to enforce.

## Two moments

1. **Setup (once per repo).** Fit the voice model and decide what shouldn't
   shape it. → `argot init`, then the **argot-setup** skill for anything with
   generated/vendored/data directories. See
   [Setup](https://argot.tmonier.com/docs/setup/).
2. **Check (per change).** Before committing code you generated or edited, score
   it. → `argot check`, or the **argot-check** skill.

## Reading `argot check`

Run `argot check --format json` for machine output. Each hit carries a
`severity`, a `reason`, an evidence trail, and a stable `hash`:

| Severity | Meaning | What to do |
|---|---|---|
| `foreign` | High-confidence anomaly — a dependency/API the repo has never used | Surface it. Reconsider whether it matches how the repo does this. |
| `suspicious` | Likely worth a look | Mention it; glance at the evidence. |
| `unusual` | Borderline | Usually fine; raise only if directly relevant. |

**Gauge trust first.** Run `argot inspect` (or MCP `argot.fit_status`). If the
verdict is **Marginal** or **Not recommended**, down-weight every hit — the model
isn't well-calibrated on this repo yet.

## When a hit is a real divergence

Look at the evidence line — it names the surprising identifier and what the repo
uses instead (`axios — 0 of 47 imports; common here: react, express, pg`). Ask:
does this match how the repo already does this? If a well-established in-voice
option exists, prefer it. If the foreign choice is deliberate (adopting a new
dependency repo-wide), **record the decision** so the noise stops:

```
argot mute <hash> --reason "adopting axios repo-wide"
```

## When a hit is a false positive

Expected — argot is statistical. Don't contort the code to satisfy it. Mute it
with a reason (committed, so it's an audit trail), or drop an inline note:

```python
# argot: ignore-next-line — vendored shim, intentional
```

See [Configure](https://argot.tmonier.com/docs/configure/) for all three
suppression surfaces.

## Never

- Never block, fail, or gate on an argot hit.
- Never mute without a real, human-meaningful reason just to silence output.
- Never add whole source directories to `.argotignore` to make it quiet — only
  exclude what genuinely isn't the repo's authored voice (generated, vendored,
  data). When unsure, ask the human.

## More

- **Skills:** `argot-setup` (local), `argot-check` (per-diff), `argot-ci` (wire
  the GitHub Action) — install with `npx skills add get-tmonier/argot`.
- **MCP** (proactive): `argot mcp` exposes `voice_context` so you can write
  in-voice from the first token — see
  [the MCP guide](https://argot.tmonier.com/docs/mcp/).
- **Docs:** <https://argot.tmonier.com/docs/> · **llms.txt:**
  <https://argot.tmonier.com/llms.txt>
