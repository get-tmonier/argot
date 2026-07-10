# P1–P4 build + real-corpus (rich) validation — RESULT

Status: **F1 (reinvention) + F4 (evidence) + F2 (placement) built, gated, tested,
and firing end-to-end.** Date: 2026-07-07. Branch: `feat/semantic-layer`.

## What shipped (all behind `feature = "semantic"`, base guardrail unchanged)

- **P1 SemanticIndex** — `callable_bodies` adapter extraction (Python + TS), per-repo
  f16 index in `.argot/semantic-index.json` (separate artifact → `scorer-config.json`
  byte-identical), `nearest`/margin query. Fit wiring embeds every corpus function.
- **P2 F1 reinvention** (`redundant`, "already implemented here") — nearest cross-file
  neighbour + margin, per-repo margin bar (98th pct of corpus self-margins), gating
  (same-file, same-name=move, dunder, test paths, trivial bodies).
- **P3 F2 placement** (`misplaced`, "unusual location") — k-NN area voting (depth-3),
  per-area belongs-fraction calibration. Fires only on strong disagreement.
- **P4 F4 evidence** — nearest-existing-code line on findings (retrieval + template,
  no LLM): `↳ duplicates <sym> (path:line) — similarity X`.
- Exit code: user chose **gating** (semantic findings flip exit like base hits).

## End-to-end proof

- **F1 fires** (synthetic testrepo): a renamed reimpl of `slugify` →
  `. already implemented here (redundant)  ↳ duplicates slugify (…:1) — similarity 0.86`.
  The genuinely-novel sibling function did NOT get a redundant finding (base
  call-receiver caught its novel callee instead) → additive, no cross-channel FP.
- **Real corpus (rich, 821 functions, fit 46 s):** a blind spec-only reimpl of
  `_cell_len` (renamed `visible_width`) — **retrieval is perfect** (nearest =
  `_cell_len` 0.71, then `cell_len` 0.66, top-6 all cell-width helpers), but F1
  **abstains**: cos₁ 0.71 / cos₂ 0.66 → **margin 0.045 ≪ bar 0.275**.
  Cause = rich has *three* cell-length functions, so no single standout — A1's
  "mutual near-duplicate dilution", now confirmed on real code. Faithful to the
  validated signal (retrieval load-bearing; margin corpus-dependent), not a bug.
  Implication: margin-firing is conservative on repos that already contain
  near-dup clusters. **Percentile tuning is a P5-bench question, not a per-example fit.**

## Two real bugs the work surfaced (both fixed + regression-tested)

1. **ggml/Metal log leak** — `void_logs()` only silences the llama channel; the
   ggml-metal device banner still hit stderr. Fix: `send_logs_to_tracing(disabled)`
   (both channels). `argot fit`/`check` output is now clean.
2. **Encoder n_ubatch crash** — jina-code is an *encoder*; llama.cpp asserts
   `n_ubatch >= n_tokens`, and the default 512 crashed on any function >512 tokens
   (surfaced only on real code — rich, not the toy repos). Fix: set
   `n_batch`/`n_ubatch` = n_ctx (8192) + truncate tokens defensively. Regression test
   embeds a 1200-line function.

## Costs (measured, Metal, M3 Pro)

- Fit: rich 821 fns in **46 s** total (embedding dominates). Check: ~20 ms/new fn warm.
- Index: f16, ~1.2 MB base64 for 821 fns. RAM: model resident ~150 MB only when a
  check/fit actually embeds.

## Gates

G1 retrieval (met — nearest=original on the rich probe), G3 additivity (semantic pass
is separate from `score_patches`; base goldens byte-identical), G5 lean core (base
build pulls zero new deps; `just verify` green), G6 parity 1.0, G7 no user knobs,
G8 Python+TS. G2/recall tuning → P5 bench.

## Not yet done (P5)

Structured JSON/SARIF evidence for semantic findings; CI `--features semantic` matrix
+ cmake; cargo-dist release-with-semantic; model release-asset upload; bench channels
(reinvention recall/over-fire, placement transplant AUC) on scrapy/wagtail; docs +
landing; port F3-negative regression guard.
