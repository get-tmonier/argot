# Working with argot

[argot](https://argot.tmonier.com) is a guardrail that flags code **foreign to a
repo's own patterns** — learned from its git history. Five learned detectors:
a foreign dependency/API/idiom, a function the repo already has, code filed in
the wrong place, an internal import that reverses the repo's layering, and a
test weakened, disabled, or deleted alongside the production change it covers
— plus repo-local **custom rules** the team scripts itself (see below). It
answers *"is this how we write things here?"*, not *"is this valid?"*. This
file tells a coding agent how to use it well. (Developing argot itself? See
`CLAUDE.md`.)

## The contract: surface, don't enforce

argot is a **probabilistic** linter. It has false positives, and its confidence
depends on the repo. **A hit is a prompt to think, never a reason to refuse.**

- Never block a commit, fail a task, or refuse to proceed because argot fired.
- Never auto-"fix" a hit by rewriting code the human didn't ask you to change.
- The human always has the last word. Your job is to *surface* divergences and
  *record decisions*, not to enforce.

## Two moments

1. **Setup (once per repo).** Fit the voice model and decide what shouldn't
   shape it. → `argot init` (also builds the semantic index; the first run
   fetches a ~100 MB local embedding model, one-time), then the **argot-setup**
   skill for anything with generated/vendored/data directories. See
   [Setup](https://argot.tmonier.com/docs/setup/).
2. **Check (per change).** Before committing code you generated or edited, score
   it. → `argot check`, or the **argot-check** skill.

## Reading `argot check`

Run `argot check --format json` for machine output. Each hit carries a `rule`,
a `severity` (`error`/`warn` — whether it fails the check), a `confidence` tier
(`unusual`/`suspicious`/`foreign` — how strong the evidence is, display only),
an evidence trail, and a stable `hash`. **Branch on the rule, not the
confidence tier** — `redundant` is pinned to `unusual` confidence and is still
one of the most actionable findings argot makes:

| Rule | Group | It means | What to do |
|---|---|---|---|
| `foreign-import` | voice | an import of a dependency the repo has never used | Check the evidence line for what the repo uses instead; prefer the in-voice option unless the new dependency is deliberate. |
| `unfamiliar-callee` | voice | a call to something this kind of file never calls | Same: compare against the named common callees. |
| `rare-tokens` | voice | a token sequence statistically foreign to the repo's voice | Read the flagged identifiers; rewrite with the repo's vocabulary if unintended. |
| `convention` | voice | a construction that breaks a learned repo convention | As above. |
| `redundant` | semantic | this new function duplicates one the repo already has | **Open the file the evidence names** (`↳ duplicates X (path:line)`), compare, and use the existing function instead — or justify and mute. |
| `misplaced` | semantic | this function's nearest kin all live in another area | Propose moving it to the named area, or justify its placement. |
| `layering` | architecture | this internal import reverses the repo's layer direction | Don't introduce the import — invert the dependency or go through the intended layer. |
| `test-deleted` | integrity | a test removed while the production code it exercised still exists | Restore the test or explain why it's obsolete; if the deletion is legitimate (feature removed), the code that exercised it should be gone too. |
| `test-disabled` | integrity | a skip/ignore marker added, or a test gutted, while production code changes | Un-skip and fix the code, or record why the skip is temporary; skipping to make a failing suite green is the exact behavior this rule exists to catch. |
| `test-weakened` | integrity | assertions removed, tautologized, or loosened while production code changes | Restore the assertion strength; if the expected value legitimately changed, say why in the commit/PR rather than silently retargeting. |

Rules are configurable like any linter: `argot rules` lists them; `argot.toml
[rules]` or `argot check --rule <name|group>=<error|warn|off>` sets severities.
Everything defaults to `error` except `test-weakened`, which ships as `warn`
(reported, never fails the check) — its finding classes are real but noisier
on routine test churn.

**Gauge trust first.** Run `argot inspect` (or MCP `argot.fit_status`). If the
verdict is **Marginal** or **Not recommended**, down-weight every hit — the model
isn't well-calibrated on this repo yet.

## What it catches — and what a clean run doesn't mean

argot reliably flags a **novel pattern** foreign to this repo (~98% when the
foreign symbol is in the change), a **reinvented function** (the evidence names
the original), **misplaced code**, and a **layering violation** (96.8% caught at
zero false positives on control edits). It also flags **tests weakened,
disabled, or deleted alongside a code change** — a test gutted, skipped, or its
assertions loosened while the production code it exercises also changes —
94% of authored gaming edits caught across 22 corpora / 11 languages, with
1.2% of real accepted test-touching commits flagged at gating severity and
zero fires on authored legitimate-refactor controls. Trust those hits.

It does **not** reliably catch *in-vocabulary* breaks — where every token is
already in the repo and only the choice is wrong (a bare `ValueError` where the
repo raises `HTTPException`). So **a clean `argot check` means "no foreign
pattern found," not "this is idiomatic."** Don't tell the user their code
matches the repo's conventions on the strength of a clean run.

## When a hit is a real divergence

Look at the evidence line — it names the surprising identifier and what the repo
uses instead (`axios — 0 of 47 imports; common here: react, express, pg`), or
the existing function a `redundant` hit duplicates. If a well-established
in-voice option exists, prefer it. If the foreign choice is deliberate (adopting
a new dependency repo-wide), **record the decision** so the noise stops:

```
argot mute <hash> --reason "adopting axios repo-wide"
```

## When a hit is a false positive

Expected — argot is probabilistic. Don't contort the code to satisfy it. Mute it
with a reason (committed, so it's an audit trail), or drop an inline note —
optionally scoped to one rule or group:

```python
# argot: ignore-next-line rule=redundant — intentional parallel implementation
```

See [Configure](https://argot.tmonier.com/docs/configure/) for all three
suppression surfaces. Housekeeping: `argot list-mutes` shows every active
suppression; `argot review-mutes` reports which ones no longer fire
(`--prune` removes them).

## Never

- Never block, fail, or gate on an argot hit.
- Never mute without a real, human-meaningful reason just to silence output.
- Never add whole source directories to `argot.toml`'s `[exclude]` to make it
  quiet — only exclude what genuinely isn't the repo's authored voice (generated,
  vendored, data). When unsure, ask the human.
- Never set a rule to `off` on your own initiative — downgrading to `warn` or
  muting a specific hit with a reason is the recorded, reversible move.

## Custom rules — the repo's own vocabulary

A repo can carry hand-written rules under `.argot/rules/<name>/` (a `rule.toml` manifest plus a
sandboxed Rhai script), committed alongside the code. `argot check` discovers and runs them
fresh every time — no rebuild. Their findings behave exactly like a built-in's: same rule name
in every output, same `[rules]`/`--rule` severities, same inline-comment and `[[mute]]`
suppressions, just under one more group, `custom`. `argot rules` lists them (with their source
directory) right after the built-ins — run it to see a repo's full rule vocabulary, built-in and
custom together.

**When a human asks you to codify a repo convention** ("we never call `X` directly," "raw SQL
isn't allowed outside the query builder"), you can write the rule yourself:

1. Create `.argot/rules/<name>/rule.toml` (`schema = 1`, `name` matching the directory,
   `languages` scoped to where the convention applies) and `check.rhai` (a `ts_query(...)` loop
   calling `report`/`report_span` — host API v1: `file`, `hunks`, `ts_query`,
   `import_attested`/`callee_attested`, `changeset_paths`).
2. Add `tests/<case>/{input.<ext>, expected.json}` fixtures — at least one that should fire and
   one that shouldn't.
3. Loop `argot rules test <name>` until every case passes, then let a real `argot check` confirm
   it fires on the intended pattern.

Full manifest schema, the host API reference, and a worked example (`no-print`, banning raw
`print()` calls) are in [Custom rules](https://argot.tmonier.com/docs/custom-rules/). Same
contract as everything else here: writing the rule is yours to do on request, but muting or
softening one of its findings is still the human's call. The **argot-write-rule** skill walks
this exact loop end-to-end, with `argot rules test <name>` as its gate.

## If the binary disagrees with this document

Trust the binary. `argot rules` prints the live rule registry and `argot
<command> --help` the live flags — this file may lag a release behind.

## More

- **Skills:** `argot-setup` (local), `argot-check` (per-diff), `argot-review-pr`
  (review one PR against the repo's voice), `argot-setup-ci` (wire the GitHub
  Action), `argot-write-rule` (codify a convention as a custom rule) —
  install with `npx skills add get-tmonier/argot`.
- **MCP** (proactive): `argot mcp` exposes `voice_context` so you can write
  in-voice from the first token — see
  [the agents guide](https://argot.tmonier.com/docs/agents/).
- **Voice guide:** `argot describe-voice --out STYLE.md` writes a
  human-readable description of the learned voice (typical callees, familiar
  imports) you can commit and point agents at.
- **History scorecard:** `argot audit` scores the last N commits against the
  voice as it was before them and attributes each finding to its introducing
  commit — ai-assisted / human / unknown, from concrete commit markers only
  (agent trailers, bot authors; never style). `--format json|markdown|html`
  for machine, PR-pasteable, or shareable output. Informational: exit 0.
- **Everything else:** `argot --help` is the full, always-current command
  surface — trust it over any list in a document.
- **Docs:** <https://argot.tmonier.com/docs/> · **llms.txt:**
  <https://argot.tmonier.com/llms.txt>
