---
name: argot-review-pr
description: Review a specific pull request (or diff range) against this repo's learned patterns with argot, without checking it out — flag dependencies, APIs, and constructs foreign to how the repo is written, duplicated functions, misfiled code, layering breaks, and tests weakened, disabled, or deleted alongside a production change. Use when the user asks to "review PR #123 with argot", "check this PR for out-of-voice code", or "run argot on that branch/range". Distinct from argot-check (your local working changes) and argot-setup-ci (wiring the GitHub Action).
---

# argot-review-pr

Score a pull request against the repo's **committed local** fit snapshot and report
what fires — argot is statistical; false positives happen. Every hit names a
**rule**, and the rule — not the confidence glyph — tells you what to
recommend. The human decides what to do with the PR.

`argot review` scores the PR's diff without checking it out, using the reviewed
fit snapshot already committed in the local repository's `.argot/`, so it is
fast and leaves the working tree untouched. Before reviewing a PR, refresh that
snapshot locally from the intended accepted branch, review and commit it — never
fit the PR head and let the change certify itself.

## Preconditions

1. `argot --version` — if missing, tell the user how to install it (see
   <https://argot.tmonier.com/docs/getting-started/>) and stop.
2. Run `argot status --format json`. The snapshot must report
   `snapshot.complete: true` and `snapshot.committed: true`. If it is missing,
   uncommitted, stale, or config-mismatched, ask for an explicit local
   `argot fit` on the accepted branch, followed by review and commit of
   `.argot/`; otherwise the result cannot represent what CI/other clones see.
3. For a PR by number/URL, the `gh` CLI must be authenticated and network access
   is required because argot fetches the PR diff through it. A locally available
   `base..head` range or commit SHA needs no network; fetch the refs first if
   they are not local.

## Run it

The target is a PR URL, `#number` / `number`, a `base..head` range, or a sha:

```
argot review 123 --format json
argot review https://github.com/org/repo/pull/123 --format json
argot review origin/main..my-branch --format json
```

Exit codes: `0` clean (or warn-severity hits only) · `1` at least one
error-severity hit · `2` setup/usage error. **Treat `1` as "there is something
to raise in the review," not as a verdict on the PR.**

Each hit in the JSON `hits` array carries `rule` (kebab-case name — branch on
this), `rule_label`, `severity` (`error` / `warn` — error hits drive exit `1`),
`confidence` (`unusual` / `suspicious` / `foreign` — evidence strength,
display-grade only), `evidence` (the lines to show — the foreign symbol and
what the repo uses instead, or the duplicated function / intended area),
`hash`, and `path` / `line_start` / `line_end`. Read `rule` and `severity`,
not the raw `score` / `threshold` (those sit on different scales per signal).
In `--format human` the meta line identifies the source and rule. PR review
findings are evidence for review, not a claim that the PR or its author is
incorrect.

## The rules

Twelve built-in rules in five groups (`argot rules` prints the registry with the
repo's effective severities — plus any scripted custom rules the repo carries
under `.argot/rules/`, group `custom`; treat their findings like any row
below, with the rule's own message as the evidence):

| Rule | Group | What it means | What to recommend |
|---|---|---|---|
| `foreign-import` | voice | Imports a dependency the repo has never used. | Point at the evidence; suggest the dependency the repo already reaches for, unless the new one is deliberate. |
| `unfamiliar-callee` | voice | Calls a receiver/callee the repo's code never calls. | Suggest the API the repo already uses, or confirm the new one is wanted. |
| `rare-tokens` | voice | A token sequence statistically foreign to the repo's voice. | Ask the author to rewrite the idiom in the repo's vocabulary if it's off-voice. |
| `convention` | voice | Breaks a convention learned from the repo. | Cite the convention from the evidence; ask for a follow-or-justify. |
| `superseded` | voice | New code uses a pattern the repo has been replacing (mined from its migration commits) or declared migrated away from (`[[migration]]` in argot.toml). Evidence cites the replacing commits or the declared reason. | Recommend the replacement the evidence names; `argot conventions` lists the migration and the files still on the old pattern. Warn by default — flag it in review, it won't gate. |
| `redundant` | semantic | Duplicates a function the repo already has — evidence `↳ duplicates <symbol> (<path>:<line>) — similarity 0.XX` names the original. | **Do not ignore.** Open the cited file, compare, and recommend calling the existing function instead of merging a reimplementation — or a justified mute. |
| `misplaced` | semantic | The function looks like it belongs in another module area — evidence `↳ looks like <area> code filed under <area>`. | Suggest moving it to the cited area, or ask the author to justify the placement. |
| `layering` | architecture | An internal import that reverses the repo's established layering direction. | Recommend not introducing the import — invert the dependency or route through the intended layer. |
| `test-deleted` | integrity | A test (or whole test file) removed while the production code it exercised still exists. | Recommend restoring the test or explaining why it's obsolete; if the deletion is legitimate (feature removed), the code that exercised it should be gone too. Call out test-gaming explicitly. |
| `test-disabled` | integrity | A skip/ignore marker added, or a test gutted, while production code changes. | Recommend un-skipping and fixing the code, or recording why the skip is temporary; skipping to make a failing suite green is the exact behavior this rule exists to catch. |
| `test-weakened` | integrity | Assertions removed, tautologized, or loosened while production code changes. | Recommend restoring the assertion strength; if the expected value legitimately changed, ask the author to say why rather than silently retargeting. |
| `rule-tampered` | governance | The diff weakens a rule the repo locked — a lock removed/downgraded, a `[[mute]]` added on a locked rule, or a locked custom rule's script edited. | **Highest priority.** This is the diff touching the guardrail itself. Pinned `error`, unsuppressable — treat any occurrence as "the change tried to disable a check", not a style nit. |

Confidence tiers grade **evidence strength only** — they never drive the exit
code (severities do), and `redundant` / `misplaced` / `layering` are always
reported at `unusual`. The three `integrity` rules (`test-deleted`,
`test-disabled`, `test-weakened`) pin to `suspicious` — each is a discrete,
evidenced event (a marker added, assertions excised), stronger than `unusual`
but not the categorical certainty of a 0-usage import. **An `unusual` hit is
NOT "usually fine" — look at its rule.** Severities: every rule defaults to
`error` except `test-weakened` and `superseded` (warn — reported, never gate
on their own); the repo can adjust per rule or group in `argot.toml` `[rules]`
or per run with `argot review --rule <name|group>=<severity>`.

## Gauge trust first

Run `argot inspect` and read the verdict. If it's **Not recommended**, the
statistical voice model isn't well-calibrated on this repo — down-weight the
`voice`-group hits accordingly and say so. **Ready — with notes** is usable
as-is; the notes say what to keep an eye on.

## What a hit means — and what a clean run doesn't

argot reliably flags a **novel pattern** foreign to this repo: a dependency it
has never imported, an API it never calls, or a whole paradigm it never writes —
plus duplicated functions, misfiled code, layering breaks, and a test weakened,
disabled, or deleted alongside the production change it covers. Trust a
`foreign-import` hit. It does **not** catch every *in-vocabulary* break — where
every token is already in the repo and only the choice is wrong. So a clean
review means "none of the configured rules fired," not "this PR is idiomatic."

## Decision tree — branch on the rule

- **`foreign-import` / `unfamiliar-callee` / `rare-tokens` / `convention`** —
  surface it, read the evidence line, and ask whether it matches how the repo
  already does this. If an in-voice option exists, suggest the switch. If the
  choice is deliberate, the reviewer can record it:
  `argot mute <hash> --reason "…"` (a committed `[[mute]]` in `argot.toml`,
  which can also target `rule = "<name|group>"`; inline form:
  `# argot: ignore-next-line rule=<name|group> — reason`).
- **`redundant`** — do **not** ignore. Open the file cited in the evidence,
  compare, and recommend using the existing function instead of the
  reimplementation — or a mute with the author's justification.
- **`misplaced`** — suggest moving the code to the area the evidence cites, or
  ask the author to justify the location.
- **`layering`** — recommend against introducing the import; invert the
  dependency or go through the intended layer. Only a deliberate architecture
  change justifies a mute.
- **`test-deleted` / `test-disabled` / `test-weakened`** — call out test-gaming
  explicitly in the review comment: a test was deleted, skipped, or weakened
  alongside a production change. Recommend restoring or un-skipping the test,
  or asking the author to justify the change (a legitimately removed feature
  should remove its test too; a legitimately retargeted expectation should say
  why). Only a justified explanation from the author warrants a mute.

## Hard rules

- **Never reject the PR unilaterally** because argot fired — argot informs the
  review, the human decides. Raise error-severity hits as review comments the
  author should fix or justify.
- **Never rewrite the PR author's code** to silence a hit. Suggest; don't
  enforce.
- **Never mute on someone's behalf** without a real reason they'd endorse.
- False positives happen. If a hit is fine, offer the `argot mute` command
  with a reason so it doesn't come back.

## If the CLI and this document disagree

If the binary reports a rule not covered by this document, trust the binary:
run `argot rules` for the registry and `argot <cmd> --help` — the CLI is the
source of truth, this skill may lag behind it.
