# `argot init` + `--suggest` — real-corpora validation

**Date:** 2026-07-06
**What:** the v1 onboarding surface — `argot init` (fit + health verdict, auto
`.argot/.gitignore`) and `argot init --suggest` (evidence-backed ignore
candidates), plus the universal setup prompt / `argot-setup` skill.
**Why:** confirm the setup flow works from scratch on real repos, that
`--suggest` fires on genuinely generated directories without over-firing on
authored source, and that writing `.argotignore` converges.

## Method

Shallow-cloned three corpora (fit/init/suggest need only the working tree, not
history) and ran `argot init` then `argot init --suggest`. For the corpus with
real generated code, wrote the suggested `.argotignore` and re-ran `--suggest`
to check convergence.

## Results

| Corpus | Lang | `init` verdict | Corpus (incl / path-excl / auto-gen) | `--suggest` |
|---|---|---|---|---|
| fastapi | Python | **Ready** | 503 / 626 / 0 | quiet (clean) |
| hono | TypeScript | **Ready** | 188 / 178 / 1 | quiet (clean) |
| grpc-go | Go | **Ready** | 743 / 264 / 33 | **4 generated dirs** |

**grpc-go `--suggest`** surfaced exactly the protobuf-generated directories, each
100% auto-generated (zero authored code dropped):

```
interop/grpc_testing              13 files · 13 auto-generated (100%)
credentials/alts/internal/proto    4 files ·  4 auto-generated (100%)
reflection/grpc_testing            4 files ·  4 auto-generated (100%)
internal/proto                     3 files ·  3 auto-generated (100%)
```

It correctly did **not** flag `examples/` (generated `.pb.go` mixed with
hand-written example code — below the 0.8 skip-ratio gate), nor promote to the
repo root. Of 33 per-file auto-generated hits, only the 24 that cluster into
cohesive directories became candidates; the ~9 lone generated files scattered
among hand-written code stayed per-file-excluded, never suggested as directory
ignores (correct — ignoring their mixed dirs would drop real code).

**Convergence:** after writing the four directories to `.argotignore`, re-running
`argot init --suggest` reported "No directories stood out" — the walk prunes
already-ignored directories, so suggestions don't repeat.

## Conclusions

- The flow works from scratch on real Python / TS / Go repos; all three fit to
  **Ready** with no hand-tuning.
- `--suggest` fires on real generated directories and stays silent on clean
  repos — no false "ignore your `src/`". The topmost-cohesive, never-swallow-real-code
  selection behaves as designed on real data, matching the unit tests.
- `.argotignore` is respected by the walk and the suggestions converge, so the
  agent/human loop (suggest → write → re-fit) terminates.
- `argot init` writes `.argot/.gitignore` (`*`), keeping the rebuildable model
  and the heavy `extract` dataset out of version control (git confirms `.argot/`
  is ignored).

Perf: `--suggest` parses each source file like `argot inspect`, so cost scales
with source count. The walk prunes `.git`, `.argot`, the `argot:recommended`
directories, and universal build/dep trees (`target/`, `node_modules/`,
`vendor/`, package caches) plus any virtualenv (detected structurally by
`pyvenv.cfg`, name-agnostic). On argot's own worst-case repo — 137k `target/`
files, 88k `benchmarks/`, a stray `.venv-phaseb`, and the legacy `engine/` tree
— a release build now runs in **~1.3 s** (was minutes before pruning). The clean
corpora above run faster.
