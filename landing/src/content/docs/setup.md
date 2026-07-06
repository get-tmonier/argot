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
coding "voice" from its history and flags future code that is foreign to it. It
only works well when it learns from code we wrote by hand — not generated code,
vendored dependencies, or pure data. Configure what argot should ignore, then
confirm the model is healthy.

1. Confirm argot is installed: run `argot --version`. If it is missing, tell me
   how to install it and stop.

2. Fit and check health: run `argot init`. Read the "Verdict" line
   (Ready / Marginal / Not recommended) and the corpus summary.

3. Get argot's statistical suggestions: run `argot init --suggest --format json`.
   These are directories that are mostly auto-generated or data files — strong
   ignore candidates, with counts.

4. Read the repository tree yourself and find directories that should NOT shape
   our voice but argot cannot detect by name:
   - generated code (protobuf/gRPC, OpenAPI/GraphQL clients, `*_pb2.py`, `gen/`)
   - vendored / third-party code checked into the repo (`vendor/`, bundled SDKs)
   - large data, fixtures, snapshots, locale tables, database migrations
   - legacy or archived modules that are not how we write code today
   Do NOT ignore our real application or library source — that is exactly what
   argot must learn from. argot already excludes tests, docs, examples, and
   build output, so you do not need to add those.

5. Write a `.argotignore` at the repo root (gitignore-style patterns, one per
   line). Add only directories you are confident about, each with a short `#`
   comment saying why. Prefer directory patterns like `src/generated/`.

6. Re-run `argot init`. Confirm the Verdict moved toward **Ready** and the
   corpus is dominated by our own code. If it is still Marginal or Not
   recommended because of a directory you can identify, refine `.argotignore`
   and repeat — at most a few rounds.

7. Summarize: what you excluded and why, and the final Verdict.

Keep it minimal and reversible — every line in `.argotignore` is a human-
readable decision I can undo.
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
