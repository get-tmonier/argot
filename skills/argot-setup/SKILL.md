---
name: argot-setup
description: Configure argot for this repository — fit the voice model and decide which directories should NOT shape the repo's voice (generated code, vendored deps, data). Writes an argot.toml and confirms the model is healthy. Use when setting up argot for the first time, when argot-check reports the repo isn't fitted, or when the user asks to "set up argot", "configure argot", or "fix argot's calibration".
---

# argot-setup

Set up argot so it learns the voice of the code **the team wrote by hand** —
not generated code, vendored dependencies, or pure data. Deciding what to
exclude is a judgment call; argot gives you statistical evidence, and you bring
the semantic knowledge of the tree.

argot fits from files as they are on disk, minus anything **gitignored** —
editor-history trees (`.history/`), local worktrees, build output and the like
never shape the voice as long as `.gitignore` covers them. Uncommitted edits to
tracked files DO count (argot warns about a dirty tree), so commit or stash
work in progress first. Prefer running setup from the **default branch**:
fitting on a feature branch bakes its unmerged commits into the voice, and
argot warns about exactly that (relay the warning to the user rather than
suppressing it).

## Steps

1. **Confirm argot is installed:** `argot --version`. If missing, tell the user
   how to install it (<https://argot.tmonier.com/docs/getting-started/>) and
   stop.

2. **Identify the primary authored source** — the library or app the repo
   actually ships. In a monorepo (multiple packages/workspaces), that's usually
   one or a few packages; everything else is peripheral.

3. **Fit and check health:** `argot init`. Read the **Verdict** line
   (Ready / Ready with notes / Not recommended) and the corpus summary. If it's already
   **Ready** with a clean corpus, you may not need to exclude anything — but
   still do step 7 (verify the catch).

   `argot init` also builds the repo's **semantic index** (for the `redundant`
   and `misplaced` rules). On first use it downloads the jina-code embedding
   model (~100 MB, one-time, to `~/.cache/argot/models/`) with a progress
   report — tell the user this up-front and don't worry about the delay. A
   failed download is verbalized, never silent. `argot model fetch` /
   `status` / `clean` manage the cache explicitly; `ARGOT_SEMANTIC_MODEL`
   points at a local gguf, `ARGOT_OFFLINE=1` skips the download, and
   `ARGOT_MODEL_URL` sets a mirror.

4. **Get argot's suggestions:** `argot init --suggest --format json`. Lists
   directories that are mostly auto-generated or data files, with counts. Note
   the `included` count (real code a rule would drop) — and that `--suggest`
   *only* finds generated/data-heavy dirs; on a monorepo it is often empty and
   the peripheral-package call in step 5 is yours.

5. **Read the tree** and find directories that shouldn't shape the voice —
   never the primary source from step 3:
   - peripheral monorepo members: a marketing/landing site, a playground, demo
     or example apps, a benchmark suite, build/dev tooling
   - generated code (protobuf/gRPC, OpenAPI/GraphQL clients, `*_pb2.py`, `gen/`)
   - **transpiled / built JavaScript in a TypeScript repo** — compiled `.js`
     output (a `.js`/`.js.map` beside a `.ts` source, or a `dist/`/`lib/`/`esm/`/
     `cjs/`/`out/` of build output). It is generated, not authored voice, and
     would pollute the JS model. argot auto-excludes the ones carrying a
     `sourceMappingURL` or tsc `__esModule` tell, but a transpiled directory with
     none of those (plain `tsc` without source maps into `lib/`) is yours to name.
   - vendored / third-party code checked into the repo (`vendor/`, bundled SDKs)
   - large data, fixtures, snapshots, locale tables, database migrations
   - legacy or archived modules that aren't how the team writes code today
   - duplicate snapshots of the repo's own code that are **committed** (an
     editor-history dir, a `backup/`/`old/` tree, copied-in worktrees) —
     gitignored ones are already skipped automatically; committed ones
     double-weight stale code and need an explicit exclude

   Sanity-check the fitted corpus: `.argot/repo-corpus.txt` lists every file
   that shaped the voice — skim it and make sure nothing surprising is there.

   argot already excludes tests, docs, examples, and build output by default —
   focus on the repo-specific dirs above.

6. **Edit `argot.toml`** at the repo root (`argot init` writes a default one).
   Add the directories you're excluding to `[exclude].paths` — gitignore-style
   patterns, one per entry, each with a trailing `# reason` comment; prefer
   directory patterns. If the repo uses a code generator whose banner isn't in
   the default `[detect].generated-markers` (e.g. a bespoke in-house codegen),
   add that phrase there too. If the user wants to soften or disable a rule
   (all default to `error` except `test-weakened`, which ships `warn`), the surface is the `[rules]` table in the
   same file — e.g. `misplaced = "warn"` or `semantic = "off"`; `argot rules`
   lists the registry with effective severities. Re-run `argot init`.

7. **Verify the catch works** — the important check. In a real primary-source
   file, add a throwaway import of a package the repo never uses (e.g.
   `import boto3` / `import axios from "axios"`) plus a line using it, run
   `argot check`, and confirm it's flagged. Then revert. If it is NOT flagged,
   the voice is still diluted by non-authored code — exclude more peripheral
   directories and repeat.

8. **Finish with the wow — audit their history:** `argot audit`. It fits the
   voice as it was ~50 commits ago (in a temp worktree — the user's tree is
   untouched), reports what argot would have caught before merge per rule
   group, and attributes each finding to its introducing commit —
   ai-assisted / human / unknown, from concrete commit markers only. Show
   the user the card. If the repo's in-scope code is younger than the window
   (a rewrite, or early history that today's excludes mute entirely), audit
   shrinks the window automatically and says so. If it says the window
   touched no supported source files, widen it (`argot audit --commits 200`
   or `--since 6m`). A quiet audit is also a result: their recent history is
   in voice.

9. **Optional — the pre-write guardrail (Claude Code):** offer to have argot
   *ask* before the agent introduces a dependency this repo has never used —
   the reviewer's "is this intentional?" beat, moved to write time. It is
   **opt-in and per-repo**; nothing fires unless you add it here. If the user
   wants it, merge this into the repo's `.claude/settings.json` (committed,
   team-shared) or `.claude/settings.local.json` (personal, gitignored) — do
   NOT overwrite an existing `hooks` block, merge into it:

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

   It fires only on a genuinely foreign dependency (argot's highest-precision
   signal), **asks** — never silently blocks — and is a no-op until the repo is
   fitted. To turn it off, remove that entry. Leave it out unless the user asks.

10. **Optional finishing artifact:** `argot describe-voice --out STYLE.md`
   generates a human-readable guide to the learned voice (typical callees per
   file cluster, the familiar import surface). Offer it when the user wants a
   committed, reviewable description of what argot learned.

11. **Summarize** for the user: what you excluded and why, and the final Verdict.
   `argot.toml` is committed (the excludes/detect/mutes are a shared, reviewable
   decision); argot also wrote a `.argot/.gitignore` so the rebuildable model
   itself isn't committed (regenerate with `argot fit`), and gitignored
   `argot.local.toml` for anyone who wants personal, uncommitted overrides.

## Recalibrating later (maintenance)

A repo drifts into mis-calibration: a new `gen/` dir, a vendored SDK, a wave of
data files. argot detects this itself — **every `argot fit`/`init` re-scans the
tree and prints a note when new generated/data-heavy directories are shaping
the voice** (it only names dirs not already excluded, so a well-configured repo
stays quiet). When the user reports argot got noisy, or that note appears:

1. `argot init --suggest --format json` — the fresh evidence.
2. Re-run steps 5–7 above (read the tree, extend `[exclude].paths`, re-fit,
   verify the catch).
3. Model freshness is NOT your job: argot auto-refits in the background when
   the fit falls ≥10 commits behind (`[fit] auto-refresh = false` disables).
   Recalibration is about *what shapes the voice*; freshness is automatic.

## Principles

- **Evidence, not orders.** `--suggest` proposes; you and the user decide. Never
  add a directory you can't justify.
- **Minimal and reversible.** Every `argot.toml` `[exclude].paths` entry is a
  human-readable decision the user can undo. When unsure whether something is
  authored voice, ask rather than exclude.
- **Don't chase a spotless verdict.** Notes are expected on small repos; the goal
  is a corpus that reflects how the team actually writes code, not a green label.

See [Configure](https://argot.tmonier.com/docs/configure/) for the full
suppression system (inline comments and durable mutes too).

If the CLI's output disagrees with this document, trust the binary: `argot
rules` and `argot <cmd> --help` are the source of truth — this skill may lag
behind them.
