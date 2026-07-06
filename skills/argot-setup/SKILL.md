---
name: argot-setup
description: Configure argot for this repository — fit the voice model and decide which directories should NOT shape the repo's voice (generated code, vendored deps, data). Writes a .argotignore and confirms the model is healthy. Use when setting up argot for the first time, when argot-check reports the repo isn't fitted, or when the user asks to "set up argot", "configure argot", or "fix argot's calibration".
---

# argot-setup

Set up argot so it learns the voice of the code **the team wrote by hand** —
not generated code, vendored dependencies, or pure data. Deciding what to
exclude is a judgment call; argot gives you statistical evidence, and you bring
the semantic knowledge of the tree.

## Steps

1. **Confirm argot is installed:** `argot --version`. If missing, tell the user
   how to install it (<https://argot.tmonier.com/docs/getting-started/>) and
   stop.

2. **Fit and check health:** `argot init`. Read the **Verdict** line
   (Ready / Marginal / Not recommended) and the corpus summary. If it's already
   **Ready**, you may be done — jump to step 6.

3. **Get argot's suggestions:** `argot init --suggest --format json`. This lists
   directories that are mostly auto-generated or data files, with counts. Strong
   ignore candidates — but note the `included` count: that's real code a rule
   would drop.

4. **Read the tree yourself** and find directories that shouldn't shape the
   voice but argot can't detect by name:
   - generated code (protobuf/gRPC, OpenAPI/GraphQL clients, `*_pb2.py`, `gen/`)
   - vendored / third-party code checked into the repo (`vendor/`, bundled SDKs)
   - large data, fixtures, snapshots, locale tables, database migrations
   - legacy or archived modules that aren't how the team writes code today

   **Do not exclude the real application or library source** — that's what argot
   must learn from. argot already excludes tests, docs, examples, and build
   output by default, so don't add those.

5. **Write `.argotignore`** at the repo root — gitignore-style patterns, one per
   line, each with a short `#` comment explaining why. Prefer directory patterns
   (`src/generated/`). Then re-run `argot init` and confirm the Verdict moved
   toward **Ready** and the corpus is dominated by the team's own code. If a
   directory you can identify is still polluting it, refine and repeat (a few
   rounds at most).

6. **Summarize** for the user: what you excluded and why, and the final Verdict.
   Note that argot wrote a `.argot/.gitignore` so the model isn't committed
   (regenerate any time with `argot fit`).

## Principles

- **Evidence, not orders.** `--suggest` proposes; you and the user decide. Never
  add a directory you can't justify.
- **Minimal and reversible.** Every `.argotignore` line is a human-readable
  decision the user can undo. When unsure whether something is authored voice,
  ask rather than exclude.
- **Don't chase a perfect verdict.** Marginal is fine for small repos; the goal
  is a corpus that reflects how the team actually writes code, not a green label.

See [Configure](https://argot.tmonier.com/docs/configure/) for the full
suppression system (inline comments and durable mutes too).
