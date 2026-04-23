# AST contrastive treelets: cross-domain collapse on rich

## Setup

After the FastAPI in-domain victory (AUC 0.9742), `ContrastiveAstTreeletScorer`
was promoted to cross-domain validation. The rich corpus was the first
test: 72 source files into `model_A`, the same stdlib `model_B`, same
depth-3 treelets, same `max` aggregation. Fixtures were 10 breaks (across
`ansi_raw`, `colorama`, `curses`, `print_manual`, `termcolor`) and 10
size-matched controls drawn from rich's own source — directly comparable
to the `bpe_contrastive_tfidf` rich run which had posted AUC 0.9900 on
the exact same fixture set. A FastAPI sanity rerun (0.9742) confirmed
the scorer was still wired correctly.

## Results

Rich overall AUC 0.2900. Every break category landed below 0.65, with
`termcolor` at AUC 0.0000 — controls literally outscored breaks in every
pair.

| category | n_breaks | AUC |
|:---|:---:|---:|
| `ansi_raw` | 2 | 0.3000 |
| `colorama` | 2 | 0.6000 |
| `curses` | 2 | 0.3000 |
| `print_manual` | 2 | 0.2500 |
| `termcolor` | 2 | 0.0000 |

The size-matched rerun controls ruled out a fixture-asymmetry artefact.
`control_console_capture` and `control_console_init` both scored 7.0513 —
above every single break in the set. Rich's own source is structurally
exotic against the stdlib (`__rich_console__` protocols, `Segment`
dataclasses, method-chained style builders), so controls drawn from
rich code scored as anomalous as the breaks.

## Interpretation

The axis that made FastAPI look dominant was measuring "structurally
unusual against the stdlib", not "structurally unusual against the
repo's own idioms". When `model_A`'s own source is already structurally
exotic, the contrast flattens and the scorer fails silently — uniformly
high scores for both classes, no threshold separates them. Corpus size
was not the explanation: 72 rich files failed in exactly the same way
as 13 click files.

---

*Source on tag `research/phase-14-pre-cleanup`:
`docs/research/scoring/signal/phase13/experiments/ast_contrastive_rich_2026-04-21.md`.
Re-written here for clarity, not copied.*
