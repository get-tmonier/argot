# tree-sitter 0.23 → 0.26 (+ latest grammars): bench-neutral, adopted for currency

**Date:** 2026-07-16
**Question:** Does moving off the parity-pinned tree-sitter 0.23 grammar set to the
current releases improve, lower, or leave unchanged the headline benchmark
(novel-pattern catch rate @ low false-alarm)?

## Change under test

- Core `tree-sitter` `0.23` → `0.26.11`.
- Grammars bumped to their latest: python/javascript/go `0.25`, rust/c/php
  `0.24`, typescript/java/cpp/ruby/c-sharp `0.23` (latest of each line).
- `tree-sitter-c-sharp` **unpinned** from `=0.23.0`: it was held because
  tree-sitter 0.23.2 (max ABI 14) rejected the ABI-15 grammar of 0.23.5; core
  0.26 accepts ABI 15, so the pin is no longer needed.

### API port (0.23 → 0.26)

- `Node::child(i)` / `Node::named_child(i)` now take `u32` (were `usize`).
  Rather than sprinkle `usize → u32` casts (lossy in theory), the ~25 index
  loops were rewritten to cursor iteration via two shared helpers
  `argot_lang::ts_parse::{child_nodes, named_child_nodes}` (the idiom already
  used by `children()`/`descendants()` in the JS/TS adapters). Traversal order
  preserved; no casts anywhere.
- `QueryCursor::matches` now returns a `StreamingIterator` (scripted-rules
  `ts_query` host call) — driven with `.next()` via the `streaming-iterator`
  crate.

## Result — flat on the headline

Honest bench, 31 corpora, identical config. Baseline = 2026-07-14 (tree-sitter
0.23); treatment = this change.

| Metric | Baseline (0.23) | Treatment (0.26) | Δ |
|---|---|---|---|
| **Gated recall (headline)** | 85.64% (620/724) | 85.64% (620/724) | **+0.00** |
| Worst over-fire (false alarm) | 1.46% | 1.46% | +0.00 |
| Foreign recall | 85.64% | 85.64% | +0.00 |
| Difficulty easy/med/hard | 360/360 · 235/245 · 25/117 | identical | +0.00 |
| Overall recall (incl. never-gated tier) | 74.28% (693/933) | 74.49% (695/933) | +0.21 |

Only three corpora moved, **none on the gated (production) tier**:

- `dagster` (Python): overall +5.0%, gated unchanged — +1 non-gated fixture.
- `gh-cli` (Go): overall +2.9%, gated unchanged — non-gated.
- `rocksdb` (C++): over-fire 0.224% → 0.045% (a hair fewer false alarms), recall
  unchanged.

Every other corpus is byte-identical. The +2 fixtures land in the secondary
tier that never gates in production.

Independently, **every parity golden (`crates/argot-core/tests/*_parity.rs`)
still passes unchanged** — the newer grammars parse the golden fixtures
identically across the tokenized surface — and `just verify` is green.

## Decision

The bump is **bench-neutral**: no headline gain, no regression. Adopted anyway
for **version currency** (staying on maintained grammars, off the ABI-14 pin)
since the cost is nil — goldens green, bench flat, one small `streaming-iterator`
dep. This record exists so the "does newer tree-sitter help the metric?"
question does not get re-chased: the answer is *no measurable effect*.
