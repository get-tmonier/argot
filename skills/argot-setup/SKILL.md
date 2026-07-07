---
name: argot-setup
description: Configure argot for this repository — fit the voice model and decide which directories should NOT shape the repo's voice (generated code, vendored deps, data). Writes an argot.toml and confirms the model is healthy. Use when setting up argot for the first time, when argot-check reports the repo isn't fitted, or when the user asks to "set up argot", "configure argot", or "fix argot's calibration".
---

# argot-setup

Set up argot so it learns the voice of the code **the team wrote by hand** —
not generated code, vendored dependencies, or pure data. Deciding what to
exclude is a judgment call; argot gives you statistical evidence, and you bring
the semantic knowledge of the tree.

argot fits from the **committed** tree (HEAD), so uncommitted edits to tracked
files are ignored — you don't need a pristine tree. A brand-new untracked file is
still read from disk, so commit or remove throwaway files first.

## Steps

1. **Confirm argot is installed:** `argot --version`. If missing, tell the user
   how to install it (<https://argot.tmonier.com/docs/getting-started/>) and
   stop.

2. **Identify the primary authored source** — the library or app the repo
   actually ships. In a monorepo (multiple packages/workspaces), that's usually
   one or a few packages; everything else is peripheral.

3. **Fit and check health:** `argot init`. Read the **Verdict** line
   (Ready / Marginal / Not recommended) and the corpus summary. If it's already
   **Ready** with a clean corpus, you may not need to exclude anything — but
   still do step 7 (verify the catch).

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

   argot already excludes tests, docs, examples, and build output by default —
   focus on the repo-specific dirs above.

6. **Edit `argot.toml`** at the repo root (`argot init` writes a default one).
   Add the directories you're excluding to `[exclude].paths` — gitignore-style
   patterns, one per entry, each with a trailing `# reason` comment; prefer
   directory patterns. If the repo uses a code generator whose banner isn't in
   the default `[detect].generated-markers` (e.g. a bespoke in-house codegen),
   add that phrase there too. Re-run `argot init`.

7. **Verify the catch works** — the important check. In a real primary-source
   file, add a throwaway import of a package the repo never uses (e.g.
   `import boto3` / `import axios from "axios"`) plus a line using it, run
   `argot check`, and confirm it's flagged. Then revert. If it is NOT flagged,
   the voice is still diluted by non-authored code — exclude more peripheral
   directories and repeat.

8. **Summarize** for the user: what you excluded and why, and the final Verdict.
   `argot.toml` is committed (the excludes/detect/mutes are a shared, reviewable
   decision); argot also wrote a `.argot/.gitignore` so the rebuildable model
   itself isn't committed (regenerate with `argot fit`), and gitignored
   `argot.local.toml` for anyone who wants personal, uncommitted overrides.

## Principles

- **Evidence, not orders.** `--suggest` proposes; you and the user decide. Never
  add a directory you can't justify.
- **Minimal and reversible.** Every `argot.toml` `[exclude].paths` entry is a
  human-readable decision the user can undo. When unsure whether something is
  authored voice, ask rather than exclude.
- **Don't chase a perfect verdict.** Marginal is fine for small repos; the goal
  is a corpus that reflects how the team actually writes code, not a green label.

See [Configure](https://argot.tmonier.com/docs/configure/) for the full
suppression system (inline comments and durable mutes too).
