# argot

> Like ESLint, but for the unwritten rules. A local JEPA learns your repo's style; `argot` flags what diverges.

A style linter that learns your codebase's voice from its own git history, then scores new code by how out-of-tune it sounds. Run it on demand, in CI, or as a pre-commit hook. Local-first, zero cloud, zero telemetry.

Built on the [JEPA](https://github.com/lucas-maes/le-wm) surprise-signal pattern: a small (~15M params) next-embedding predictor trained on your repo's past commits. When a new hunk is far from what the model expected, `argot` raises its hand.

## What it catches

| Signal | What it means |
| --- | --- |
| **LLM paste-through** | A block whose style diverges sharply from the surrounding file |
| **Convention drift** | Error handling, logging, or patterns that don't match the repo |
| **Debug leftovers** | `console.log`, `.only`, commented-out code you never normally commit |
| **Stylistic outlier** | New code that's correct, but stylistically foreign to this codebase |

It does *not* replace linters like ESLint or ruff, type checkers, or security scanners. It catches what they can't: things that are *technically fine but socially wrong* for this project.

## Quick start

```bash
# install
bun add -g argot               # or: brew install argot (eventually)

# train a model on this repo's history (one-time)
cd my-project
argot train                    # ~15–30 min on a consumer GPU

# lint your staged changes
git add .
argot check                    # lists the most surprising hunks

# lint a specific range
argot check HEAD~5..HEAD
argot check --files src/foo.ts src/bar.ts
```

The model lives in `.argot/model.ckpt`, gitignored by default.

## Where it runs

`argot` is a plain CLI. Wire it wherever linters go:

- **On demand**: `argot check` from your terminal, same as `eslint .` or `knip`.
- **Pre-commit**: `argot install-hook` wires it into `.git/hooks/pre-commit`. Opt-in, not the default.
- **CI**: add a step that runs `argot check ${{ github.event.pull_request.base.sha }}..HEAD` and fails the build above a threshold.
- **Editor**: exit code + JSON output (`argot check --json`) make it trivial to wrap in a VS Code extension or LSP (planned, not shipped).

## How it works

1. `argot train` walks your git log, extracts commit diffs, tokenizes them with a language-aware tokenizer, and trains a small JEPA to predict the embedding of each hunk given its surrounding context.
2. `argot check` runs the same pipeline on the target diff and scores each hunk by *prediction error* — the surprise signal. High surprise means "this doesn't sound like the rest of the repo."
3. Output is a ranked list of hunks + a per-hunk score. Exit code non-zero if any hunk exceeds the configured threshold.

No code ever leaves your machine. The model is specific to your repo and useless to anyone else.

## Calibration

Surprise is relative. `argot` uses a rolling percentile calibration: a hunk is flagged only if it's in the top X% of surprising hunks among your recent commits. Tune with `argot config set threshold 95`.

## Limitations

- Needs meaningful history (~200+ commits) to be useful. Below that, `argot` will warn and abstain.
- Best on codebases with a consistent hand. Highly polyglot repos with ten different styles are harder to model.
- Cold start on brand-new files: the model has less context to rely on.
- The signal is noisier on very small hunks (< 5 lines).

## Stack

- **CLI**: TypeScript + Bun
- **Training**: Python + PyTorch (JEPA implementation adapted from LeWM)
- **Tokenizer**: tree-sitter for language-aware splitting

## License

MIT