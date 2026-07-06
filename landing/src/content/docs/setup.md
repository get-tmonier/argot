---
title: Setup
description: Fit argot to your repo in one command — or let an AI agent decide what shouldn't shape your voice.
group: Start
order: 2
---

argot learns the voice of the code **you** wrote by hand. Setup is really one
question: *what should it learn from?* Everything hand-written stays in; anything
generated, vendored, or pure data should stay out — otherwise argot learns a
voice that isn't yours and flags the wrong things.

There are three ways in, fastest first.

## 1. One command

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
that's all you need. If the verdict is **Ready**, you're done.

`init` also drops a `.argot/.gitignore` so the fitted model — a rebuildable
artifact — never lands in version control.

## 2. See what stands out

If the verdict is **Marginal** or **Not recommended**, the corpus is usually
either too small or polluted by generated/data directories argot can't recognize
by name. Ask for evidence:

```text
argot init --suggest
```

```text
Directories you may want to add to .argotignore (evidence only — you decide):

  src/generated
    auto-generated · 214 files · 214 auto-generated (100%)
  api/openapi
    auto-generated · 38 files · 36 auto-generated (95%) · 2 non-generated files would be dropped
```

These are directories that are *mostly* generated or data — strong candidates,
but the call is yours: the report tells you exactly how much real code a rule
would drop.

## 3. Let an AI agent set it up  <span class="rec">recommended</span>

Deciding what *shouldn't* shape your voice is a judgment call — a vendored
`stripe/` client, an OpenAPI SDK, a `legacy/` module frozen years ago. An agent
that can read your tree makes that call well, and argot's `--suggest` gives it
hard evidence to anchor on. Paste this into Claude Code (or Cursor, Aider, any
agent) at your repo root:

```text
You are setting up **argot** for this repository. argot learns this repo's own
coding "voice" from its source and flags future code foreign to it. It only
works well when it learns from the code we actually ship and maintain by hand —
not demos, generated code, vendored dependencies, or config. Configure what
argot ignores, confirm the model is healthy, and verify it catches a foreign
import.

First, work on a clean tree: `argot init` learns from files as they are, so
commit or stash any work in progress (uncommitted foreign code would be baked
into the voice).

1. Confirm argot is installed: `argot --version`. If missing, tell me how to
   install it and stop.

2. Identify the PRIMARY authored source — the library or app this repo actually
   ships. In a monorepo (multiple packages/workspaces), that's usually one or a
   few packages; everything else is peripheral.

3. Fit and check health: `argot init`. Read the "Verdict" and corpus summary.

4. Get argot's statistical suggestions: `argot init --suggest --format json` —
   directories that are mostly auto-generated or data files, with counts. Note:
   `--suggest` only finds generated/data-heavy dirs; on a monorepo it is often
   empty, and the peripheral-package call in step 5 is yours to make.

5. Read the tree and exclude what should NOT shape our voice — but never the
   primary source from step 2:
   - peripheral monorepo members: a marketing/landing site, a playground, demo
     or example apps, a benchmark suite, build/dev tooling
   - generated code (protobuf/gRPC, OpenAPI/GraphQL clients, `*_pb2.py`, `gen/`)
   - vendored / third-party code checked in (`vendor/`, bundled SDKs)
   - large data, fixtures, snapshots, locale tables, database migrations
   - legacy or archived modules that aren't how we write code today
   argot already excludes tests, docs, examples, and build output by default, so
   focus on the repo-specific directories above.

6. Write a `.argotignore` at the repo root (gitignore-style, one pattern per
   line, each with a short `#` reason). Prefer directory patterns. Re-run
   `argot init`.

7. VERIFY the catch works — the important check. In a real primary-source file,
   add a throwaway import of a package this repo never uses (e.g. `import boto3`
   / `import axios from "axios"`) plus a line using it, run `argot check`, and
   confirm it's flagged. Then revert. If it is NOT flagged, the voice is still
   diluted by non-authored code — exclude more peripheral directories and repeat.

8. Summarize what you excluded and why, and the final Verdict.

Don't chase a green "Ready". If the verdict stays **Marginal** only because the
repo is small (few candidate hunks), that's fine — Marginal is usable and
excluding more won't help. Keep excluding only when the corpus is polluted by
non-authored code (step 7 reveals this). Keep it minimal and reversible — every
`.argotignore` line is a decision I can undo.
```

The agent runs the same commands you would; it just brings the semantic
judgment argot deliberately leaves to a human. When it finishes you'll have a
committed `.argotignore` that documents every exclusion, and a **Ready** model.

## 4. By hand

Prefer to drive it yourself? [Configure](/docs/configure/) documents
`.argotignore`, the `argot:recommended` defaults, inline comments, and durable
mutes in full. The whole system is plain text — nothing here needs an agent.

---

Once the verdict is **Ready**, score your changes with
[`argot check`](/docs/the-commands/), wire it into
[CI](/docs/ci/), or point your coding agent at it via
[skills and MCP](/docs/agents/).
