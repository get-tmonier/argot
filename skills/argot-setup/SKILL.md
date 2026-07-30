---
name: argot-setup
description: Set argot up for a repository end to end — audit its history, decide what should shape its voice, fit, verify the fit actually catches things, tune the rules its own history says are noisy, and wire the places it runs (pre-write hook, pre-commit, MCP, CI). One sitting, one decision at a time, each proposed with the measurement behind it. Use for first-time setup, when argot-check says the repo isn't fitted, when the user asks to "set up argot", "configure argot", "add argot to CI", or "fix argot's calibration".
---

# argot-setup

Argot learns how *this* repository writes code and flags what looks foreign to
it. Everything downstream depends on one judgment: **which code is the voice**.
Get that wrong and it fails silently — false alarms people learn to scroll past,
or real problems never flagged. Neither looks like an error.

So this is not a wizard to click through. Every step below proposes something
**with the number that motivates it**, and the user decides. Never apply an
exclusion you cannot justify in one sentence.

Work through the phases in order. Any of them can be answered "skip".

---

## 0 · Preflight

- `argot --version`. Missing → point at
  <https://argot.tmonier.com/docs/getting-started/> and stop.
- A git repository with real history. Argot has nothing to learn from a fresh
  `git init`.
- **Default branch, clean tree.** Fitting a feature branch bakes its unmerged
  commits into the voice; a dirty tree learns files as they sit on disk. Argot
  warns about both — relay the warning, never suppress it.

Say what setup will write: `argot.toml` (committed — the excludes and rule
choices are a team decision), `.argot/` (gitignored, rebuildable), and whatever
integrations get chosen in phase 8. `argot uninstall --dry-run` lists everything
that would be removed, if they want to know the exit before the entrance.

## 1 · Proof before configuration

```sh
argot audit --format json > /tmp/argot-audit.json    # keep it: phase 6 reads it
```

No configuration needed, exits 0, touches nothing. It fits the voice as it was
~50 commits ago in a temporary worktree and reports what would have met review
since, attributed to the introducing commit. **Show the card.** This is the
honest opener: here is what your own history says, before you change anything.

**Say what it will cost before you start it, or they will kill it.** Audit fits
the whole corpus once, so a large repository costs minutes and gigabytes, not
seconds — 924k lines of Object Pascal took 2 min 38 s and 3.6 GB. A narrower
`--commits` does **not** make it cheaper: the window only moves the base
commit, and the fit still reads every file. The lever is `[exclude]`, which is
phase 2 — so on a repository this size, offer to do the obvious exclusions
first and audit second.

**Then check it produced something**: exit status 0 *and* a non-empty JSON
file. A run that dies leaves a 0-byte file behind, and an empty audit reads
exactly like a clean one.

A quiet audit is a result too — their recent history is in voice. If the window
touched no supported source, widen it: `--commits 200` or `--since 6m`.

Keep the card — phase 6 reads this same file, so do not pay for a second run.

## 2 · Scope — what should shape the voice

Three sources of evidence, merged into **one** proposal list:

1. `argot init --suggest --format json` — two kinds of directory, each with the
   evidence behind it in `reason`:
   - `auto-generated` / `data-dominant` / `generated + data` — mostly machine
     output. Note the `included` count: real code a blanket rule would drop.
   - `not-authored-here` — the repo **stores** it but does not **write** it: a
     vendored library, a forked upstream copy, a machine-translated binding.
     `edit_ratio` is how cold it is against the repo's own average (`0.04` =
     touched a twenty-fifth as often) and `source_lines` how much voice the
     entry would remove. These are usually the biggest wins on the list, and
     no amount of reading finds them faster than the ratio does.
2. **Read the tree yourself.** `--suggest` reads history and file contents, not
   intent — and it deliberately stays silent on a directory the repo *does*
   maintain, however foreign its origin. Look for:
   - vendored code the repo keeps patching (upstream forks it has adopted)
   - demo, example, playground or sample trees
   - peripheral monorepo members: a landing site, a benchmark suite, dev tooling
   - generated clients (protobuf/gRPC, OpenAPI/GraphQL, `*_pb2.py`, `gen/`)
   - **transpiled JS in a TypeScript repo** — a `.js` beside its `.ts`, or a
     `dist/`/`lib/`/`esm/`/`out/` of build output. Argot auto-excludes the ones
     carrying a `sourceMappingURL` or `__esModule` tell; plain `tsc` output with
     neither is yours to name.
   - committed duplicate snapshots (`backup/`, `old/`, an editor-history tree).
     Gitignored ones are already skipped; committed ones double-weight stale code.
   - large data, fixtures, snapshots, locale tables, migrations

Check before committing to it:

```sh
argot inspect --corpus      # every file that will shape the voice, pre-fit
```

**Monorepo fork.** Excluding a package is not the only option. If sub-trees are
genuinely different codebases rather than peripheral ones, calibrate them
separately instead — a per-slice threshold judges each area against its own
distribution. Exclude what is *not the team's voice*; slice what *is, but
differs*.

Write the decisions into `argot.toml` `[exclude].paths`, gitignore-style, one
per line, **each with a trailing `# reason`**. That comment is what makes the
choice reviewable in the PR that adds it.

## 3 · Fit, then gate on health

```sh
argot init
```

Fits the voice, writes `argot.toml` if absent, and builds the semantic index.
The embedding model ships inside the binary, so nothing is downloaded and this
works on a machine with no network.

Then read the health **programmatically**:

```sh
argot inspect --format json
```

- `verdict: "not_recommended"` → **stop and go back to phase 2.** Do not proceed
  to a check nobody should trust.
- `reasons[].signal == "voice_not_where_the_work_is"` → argot has found a
  directory that shapes a large share of the voice but takes almost none of the
  recent changes. That is the classic mis-scope: a model learned from code
  nobody edits, judging the code everybody does. Its message names the
  directory and both shares — take it to phase 2.
Also read what `argot init` **printed** — evidence phase 2 could not have had:

- *"N files argot:recommended would exclude are shaping the voice (…)"* — config
  and tooling files that slipped in. Add every path it names and refit.
- *"`<language>`: N files — too few to learn a voice"* — those files are **not
  checked at all**. Say so. If an extension is routed to the wrong language
  here, exclude it.

Scoping is two passes, not one; being sent back to phase 2 is the flow working.

- Yellow notes on a small repo are expected. **Don't chase a spotless verdict**;
  the goal is a corpus that reflects how the team writes code.

## 4 · Verify it actually catches something

A fit that flags nothing is indistinguishable from a healthy repo until it
matters. Prove the catch:

- **Choose the package deliberately, don't guess.** `argot conventions` (or
  `.argot/repo-corpus.txt`) shows what this repo actually imports; pick
  something plainly outside it. A badly chosen fixture proves nothing either way.
- In a **primary-source** file — not a test, not an example — add that import
  plus a line using it. `argot check`. Confirm it fires. Revert.
- If it does **not** fire, do not conclude the tool is weak. Run the diagnosis
  below first.

## 5 · Adoption — baseline, or clean slate?

On an existing codebase `argot check` will surface pre-existing findings. Ask
outright, because the answer shapes everything after:

- **Baseline** — `argot check --add-ignores` writes an inline ignore above
  every current finding. From here only *new* code is judged. Right for a mature
  repo adopting argot without a cleanup project first.
- **Clean slate** — fix or mute them now. Right for a smaller or younger repo.

If they baseline, say the follow-up out loud: those ignores are a snapshot, and
argot can re-score muted files and report which suppressions no longer fire.
Schedule that, or the baseline quietly becomes permanent.

## 6 · Tune from the repository's own history

Re-read the audit JSON saved in phase 1. Its `over_firing` lists
rules that trip **more than 2 % of scanned hunks**. A healthy repository sits at
0.3–0.7 % per rule, so anything above that bar is describing the repository
rather than flagging it.

For each, propose in this order — never jump to `off`:

1. **Scope it** to the tree that trips it —
   `foreign-import = { severity = "error", exclude = ["**/*.bench.ts"] }`.
   Right when the rule is correct everywhere but one place.
2. **Soften it** to `warn` — reported, never fails a check.
3. **Accept it** with a standing `[[mute]]` and a reason.

**Migrations.** If the audit shows one dependency steadily replacing another —
or the user says they are mid-migration — declare it. Two lines, effective
immediately, no refit:

```toml
[[migration]]
from = "moment"
to = "date-fns"
reason = "Q2 date-handling refactor"
```

The `to` side stops reading as foreign, the `from` side raises `superseded` in
new code, and `argot conventions` lists what is left to migrate.

**If the user wants tests checked.** By default tests are neither learned from
nor scored. To guard them, remove the test patterns from `[exclude].recommended`
and leave them in `[exclude].check-only`: argot then learns their *dependency
vocabulary* (a library only tests use stops reading as foreign) but never their
*style*. Needs a refit. Do **not** reach for `foreign-import = { exclude = [...] }`
here — that discards the real signal of a test grabbing a brand-new dependency.

## 7 · Conventions — a shortlist, not a dump

```sh
argot conventions --format json
```

Lists what the repo already does: naming, syntax idioms, familiar imports,
typical calls per area. **Do not relay it wholesale.** Present at most a handful,
ranked by one criterion: *would a rule for this have caught something in the
audit window?* A convention that is strongly attested, mechanically checkable,
and has real violations in recent history is worth a rule. Everything else is
noise at setup time.

Hand any they pick to **argot-write-rule**.

## 8 · Where it runs — decide local and CI together

Ask once, covering both. These are not separate projects.

**Local**

- **Pre-write guardrail** — argot *asks* before an agent introduces a dependency
  the repo has never used. Non-blocking, and a no-op until fitted. If the user
  has the Claude Code plugin **it is already there** — say so and move on.
  Otherwise merge into `.claude/settings.json` (team) or
  `.claude/settings.local.json` (personal) — merge into any existing `hooks`
  block, never overwrite it:

  ```json
  {
    "hooks": {
      "PreToolUse": [
        {
          "matcher": "Write|Edit|MultiEdit",
          "hooks": [
            { "type": "command", "command": "argot hook --repo \"${CLAUDE_PROJECT_DIR}\"", "timeout": 10000 }
          ]
        }
      ]
    }
  }
  ```

  Do not add it alongside the plugin — it would run twice.

- **pre-commit** — `argot-check` scores staged changes and is advisory;
  `argot-check-gate` blocks on error-severity findings. Argot must be on PATH
  (the framework will not install a static binary) and the repo must be fitted.

- **MCP** — `argot mcp` serves read-only `voice_context`, `conventions`,
  `check`, `explain`, `fit_status` to any MCP client, so an agent can ask what
  the repo's voice *is* before writing. Passive: connecting it never triggers a
  check.

**CI** — the GitHub Action, non-blocking by default:

```yaml
name: argot
on:
  pull_request:
  push:
    branches: [main]   # the run that refreshes the fitted artifacts PRs reuse

permissions:
  contents: read
  pull-requests: write     # the sticky findings comment
  security-events: write   # SARIF code-scanning annotations

jobs:
  voice:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0   # argot fits on the PR's base branch
      - uses: get-tmonier/argot@main
```

Two things must be said, or the tool gets judged on its worst run:

1. **Keep both triggers.** Fitting is nearly the whole cost; the check is
   seconds. The `push` run on the default branch is the *producer* — it fits and
   publishes the resulting `.argot/` artifacts into a cache slot, after merge,
   on nobody's critical path. Each `pull_request` is a *consumer* and pays
   nothing. Drop the `push` trigger and every PR pays the fit.
2. **The cache only exists once this workflow is on the default branch.** Until
   it is merged, every PR run is a cold fit and looks slow. Tell them before
   they conclude argot is slow — the first run is the slow one, by design.

Do not add `fail-on-hits: true` unless they explicitly want a merge gate.

## 9 · Prove it end to end

Offer, never impose:

- **Local smoke check** — keep phase 4's fixture as a command they can re-run.
Keep it local. Do not push branches or open pull requests on the user's repo to
demonstrate the tool.

## 10 · Summarize

- what the audit found — the highlights, not the dump
- what was excluded and why (the one-sentence reasons)
- the health verdict and any remaining notes
- which integrations were wired, and what the team sees on their next PR
- **maintenance:** refits are automatic when the fit falls behind
  (`[fit] auto-refresh = false` disables). *Re-scoping is not* — when a new
  `gen/` or vendored tree appears, argot names it at the next fit; act on it.

Optional: `argot describe-voice --out STYLE.md` writes a committed,
human-readable description of what argot learned.

---

## Diagnosis — read this before judging argot's quality

If argot seems noisy, or seems to catch nothing, it is far more often **mis-scoped
than wrong**. This matters because a mis-configured argot fails *silently*: no
error, just false alarms the team learns to scroll past, or real problems never
flagged. Neither is visible in the findings themselves — you have to ask.

**Always run these before concluding anything about the tool:**

```sh
argot inspect --format json     # is the model sound?
argot audit  --format json      # is a rule describing the repo rather than flagging it?
```

| what to read | what it means |
|---|---|
| `verdict: "not_recommended"` | don't trust a finding yet — fix scope first |
| `reasons[].signal == "voice_not_where_the_work_is"` | **the most common cause of both noise and silence.** A directory shapes much of the voice but takes almost none of the recent changes: the model learned from code nobody edits and is judging the code everybody does. The message names the directory and both shares. Fix the scope — never tune rules around it |
| `reasons[].signal == "polyglot_mix"` | several languages share the repo; expect weaker signal |
| `over_firing[]` in the audit | rules tripping **>2 % of the repo's own accepted history**. A healthy repo sits at 0.3–0.7 % per rule. Above that the rule is describing this repository, not flagging it — scope or soften it (phase 6), don't read it as argot being inaccurate |
| `unlearnable_languages` in `.argot/manifest.json` | those files are **not checked at all**. Silence there means *not looked at* |
| *"N large hunk(s) scored above the flat threshold but not above the one for their size"* | the score is a max over the hunk's tokens, so a big hunk scores higher for free; the bar rises with size past the repo's own p90. Those hunks were **judged against a higher bar, not skipped** — review them by hand if they are rewrites |

Relay every one of these to the user verbatim. A skipped check that reads as a
passed check is the failure mode this whole flow exists to prevent.

## Suppressing a finding

Two forms, and the difference matters:

- `argot mute <hash>` — a **per-hit** acceptance. Covers that hit only; the same
  finding in a sibling file has its own hash. Right for a genuine one-off.
- `argot mute --path 'src/legacy/**' --rule foreign-import --reason '…'` — a
  **standing** decision covering every future hit under the glob. Right for a
  tree with a known, accepted exception.

Reaching for the first when you meant the second is how a repo ends up with one
committed mute per file.

## Principles

- **Evidence, not orders.** Every proposal carries its number. The user decides.
- **Minimal and reversible.** Each `[exclude].paths` entry is a readable
  decision someone can undo.
- **Don't chase a spotless verdict.** Notes are normal on small repos.
- **A skipped check is not a passed check.** When argot reports it did not judge
  something — a language below the file floor, an oversized hunk, a rule
  disabled — relay it. Silence that means *not checked* must never be read as
  silence that means *nothing found*.

If the CLI disagrees with this document, trust the binary: `argot rules` and
`argot <cmd> --help` are the source of truth. Full reference:
[Configure](https://argot.tmonier.com/docs/configure/) ·
[llms.txt](https://argot.tmonier.com/llms.txt).
