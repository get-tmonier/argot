---
title: Setup
description: Fit argot to your repo in one command — or let an AI agent decide what shouldn't shape your voice.
group: Start
order: 2
---

argot learns the voice of the code **you** wrote by hand. Setup is really one
question: *what should it learn from?* Everything hand-written stays in; anything
generated, vendored, or pure data should stay out — otherwise argot learns a
voice that isn't yours and flags the wrong things. (Gitignored files are
already out — the fit skips anything git doesn't consider part of the repo,
so editor-history trees and build output never need an exclude.)

This is the **local path** — checking on your machine (and in a pre-commit hook).
Just want a voice score on every PR, with no local install? That's the
[CI path](/docs/ci/) instead. Four ways in, fastest first — the first hands the
judgment call to your coding agent; the rest are yours to drive.

## 1. Let your agent set it up  <span class="rec">recommended</span>

Deciding what *shouldn't* shape your voice is a judgment call — a vendored
`stripe/` client, an OpenAPI SDK, a `legacy/` module frozen years ago. The
**argot-setup** skill hands that call to the agent already reading your tree: it
runs `argot init`, weighs the `--suggest` evidence, writes the excludes into
`argot.toml`'s `[exclude].paths`, and verifies the catch — driving the same
`argot` binary you would, with nothing to copy-paste. Install it once:

```text
npx skills add get-tmonier/argot
```

Then run **`/argot-setup`** in Claude Code or Cursor (Codex: `$argot-setup`; the
skill works across 70+ agents). In Claude Code you can instead install the
plugin, which bundles the skills *and* the MCP server:

```text
/plugin marketplace add get-tmonier/argot
/plugin install argot@argot
```

The skill runs the exact prompt in §3 for you — reach for that prompt directly
only on an agent where you can't install the skill.

## 2. One command yourself

```text
argot init
```

`init` fits the model and prints a health check:

```text
Verdict: Ready
Next:  argot check          # score your working changes
```

Out of the box argot already ignores tests, docs, examples, build output, and
any file it detects as auto-generated or data-only (the built-in
`argot:recommended` set — see [Configure](/docs/configure/)). For a lot of repos
that's all you need. If the verdict is **Ready**, you're done. `init` also writes
an `argot.toml` with the effective `[exclude]`, `[detect]`, and `[[mute]]` sections
spelled out (so nothing is hidden), and drops a `.argot/.gitignore` so the fitted
model — a rebuildable artifact — never lands in version control. During fit it also
builds the semantic layer's code-embedding index (`.argot/semantic-index.json`) so
the reinvention and placement checks are ready on your first `check` — no extra step.
The first fit downloads the ~100 MB embedding model to a shared local cache, once per
machine (pre-fetch it with `argot model fetch`, or skip it offline — argot says so and
carries on; see [Configure](/docs/configure/#environment-variables)).

### If the verdict isn't Ready

If it's **Marginal** or **Not recommended**, the corpus is usually either too
small or polluted by generated/data directories argot can't recognize by name.
Ask for evidence:

```text
argot init --suggest
```

```text
Directories you may want to add to argot.toml [exclude].paths (evidence only — you decide):

  src/generated
    auto-generated · 214 files · 214 auto-generated (100%)
  api/openapi
    auto-generated · 38 files · 36 auto-generated (95%) · 2 non-generated files would be dropped
```

These are directories that are *mostly* generated or data — strong candidates,
but the call is yours: the report tells you exactly how much real code a rule
would drop. Add the ones you agree with to `argot.toml`'s `[exclude].paths` (§4)
and re-run `argot init`.

## 3. The copy-paste prompt

No skills CLI on your agent? Paste this **local-setup** prompt into Claude Code
(or Cursor, Aider, any agent) at your repo root — it's exactly what the
`argot-setup` skill (§1) runs for you, so use it only when you can't install the
skill. There's a matching [CI-setup prompt](/docs/ci/) for the CI path:

```text
You are setting up **argot** for this repository. argot learns this repo's own
coding "voice" from its source and flags future code foreign to it. It only
works well when it learns from the code we actually ship and maintain by hand —
not demos, generated code, vendored dependencies, or config. Configure what
argot ignores, confirm the model is healthy, and verify it catches a foreign
import.

argot fits from files as they are on disk, minus anything **gitignored** —
editor-history trees, local worktrees, and build output never shape the voice
as long as `.gitignore` covers them. Uncommitted edits to tracked files DO
count (argot warns on a dirty tree), so commit or stash work in progress
first, and prefer running this from the **default branch** — fitting on a
feature branch bakes its unmerged commits into the voice (argot warns about
that too; relay the warning to me rather than suppressing it).

1. Confirm argot is installed: `argot --version`. If missing, tell me how to
   install it and stop.

2. Identify the PRIMARY authored source — the library or app this repo actually
   ships. In a monorepo (multiple packages/workspaces), that's usually one or a
   few packages; everything else is peripheral.

3. Fit and check health: `argot init`. Read the "Verdict" and corpus summary. If
   it's already **Ready** with a clean corpus, you may not need to exclude
   anything — but still verify the catch (step 7).

4. Get argot's statistical suggestions: `argot init --suggest --format json` —
   directories that are mostly auto-generated or data files, with counts. Note:
   `--suggest` only finds generated/data-heavy dirs; on a monorepo it is often
   empty, and the peripheral-package call in step 5 is yours to make.

5. Read the tree and exclude what should NOT shape our voice — but never the
   primary source from step 2:
   - peripheral monorepo members: a marketing/landing site, a playground, demo
     or example apps, a benchmark suite, build/dev tooling
   - generated code (protobuf/gRPC, OpenAPI/GraphQL clients, `*_pb2.py`, `gen/`)
   - transpiled / built JavaScript in a TypeScript repo — compiled `.js` output
     (`dist/`/`lib/`/`esm/`/`cjs/`/`out/`); it is generated, not authored voice.
     argot auto-excludes the `.js` carrying a `sourceMappingURL`/`__esModule`
     tell, but a plain-`tsc`-into-`lib/` build with none of those is ours to name
   - vendored / third-party code checked in (`vendor/`, bundled SDKs)
   - large data, fixtures, snapshots, locale tables, database migrations
   - legacy or archived modules that aren't how we write code today
   - COMMITTED duplicate snapshots of our own code (an editor-history dir, a
     `backup/`/`old/` tree) — gitignored ones are already skipped automatically
   argot already excludes tests, docs, examples, and build output by default, so
   focus on the repo-specific directories above. Sanity-check the result:
   `.argot/repo-corpus.txt` lists every file that shaped the voice — skim it
   and make sure nothing surprising is there.

6. Edit `argot.toml`'s `[exclude].paths` at the repo root — add each directory
   as a gitignore-style pattern (one per array entry, each with a trailing
   `# reason` comment). Prefer directory patterns. If the repo has its own
   codegen banner, add it to `[detect].generated-markers` too. Re-run
   `argot init`.

7. VERIFY the catch works — the important check. In a real primary-source file,
   add a throwaway import of a package this repo never uses (e.g. `import boto3`
   / `import axios from "axios"`) plus a line using it, run `argot check`, and
   confirm it's flagged. Then revert. If it is NOT flagged, the voice is still
   diluted by non-authored code — exclude more peripheral directories and repeat.

8. Finish with the proof on our own history: `argot replay`. It fits the voice
   as of ~50 commits ago in a temp worktree (our tree stays untouched) and
   reports what argot would have caught before merge — show me the report. If
   the in-scope code is younger than the window, replay shrinks it
   automatically and says so; a quiet replay is also a result.

9. Summarize what you excluded and why, and the final Verdict.

Don't chase a green "Ready". If the verdict stays **Marginal** only because the
repo is small (few candidate hunks), that's fine — Marginal is usable and
excluding more won't help. Keep excluding only when the corpus is polluted by
non-authored code (step 7 reveals this). Keep it minimal and reversible — every
`[exclude].paths` entry is a decision I can undo.
```

The agent runs the same commands you would; it just brings the semantic
judgment argot deliberately leaves to a human. When it finishes you'll have a
committed `argot.toml` that documents every exclusion, and a **Ready** model.

## 4. By hand

Prefer to drive it yourself? [Configure](/docs/configure/) documents
`argot.toml`, the `argot:recommended` defaults, inline comments, and durable
mutes in full. The whole system is plain text — nothing here needs an agent.

---

Once the verdict is **Ready**, score your changes with
[`argot check`](/docs/the-commands/), wire it into
[CI](/docs/ci/), or point your coding agent at it via
[skills and MCP](/docs/agents/).
