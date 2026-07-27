# `[exclude].check-only` — bench parity (A/B, 35 corpora)

**Question.** Does the `check-only` scope work (commit 49ccb582, the one-PR fix for
the TS-monorepo defect report) move the north-star metric?

**Method.** Two full honest-bench runs on the same machine, same day, same flags
(`--jobs 4`, mode `honest`, 35 corpora / 12 languages), differing only in the
binary:

- baseline: `argot-bench` built at `0c734263` (v0.2.106, before the change), in a
  throwaway worktree with its own `CARGO_TARGET_DIR`
- patched: `argot-bench` built at `49ccb582`

Comparing against the committed `benchmarks/results/latest` dashboard was rejected:
it predates Pascal (#147), the conventions pass (#150), supersession (#151), the
real-world hardening (#146) and tree-sitter 0.26 (#135), so it cannot attribute a
delta to this change.

**Result: flat.** Every total identical, and every one of the 35 per-corpus records
byte-identical.

| metric | baseline `0c734263` | patched `49ccb582` |
|---|---|---|
| gated / foreign recall | 641/746 = 85.9249% | 641/746 = 85.9249% |
| total recall | 717/963 = 74.4548% | 717/963 = 74.4548% |
| worst over-fire, existing file | 13.8316% | 13.8316% |
| worst over-fire, new file | 32.9268% | 32.9268% |
| foreign by difficulty | 372/373 · 244/254 · 25/117 | identical |

**Why flat was expected, and what the run adds.** The bench harness builds its own
`SequentialConfig` with `check_only_patterns: Vec::new()`, so every check-only branch
is dead code inside it; and the learned corpus was separately proven identical (see
below). What the A/B adds is coverage of the *replay* side — candidate collection and
the temporal-holdout windows — which the model-hash argument does not reach.

**Corpus-level parity (separate check, 8 corpora / 6 languages).** `repo-corpus.txt`
and every language's `model_hash` compared between the two binaries after
`git reset --hard && rm -rf .argot`: fastapi, hono, cobra, bat, guava, laravel,
rubocop, rocksdb — all identical. This is what caught the one real regression during
development: a bare `test_*` in the default check-only list matched any path
*component*, swallowing rocksdb's `test_util/` (15 files of production support code).
Fixed with `IgnorePattern::matches_file_scoped`; pinned by
`corpus.rs::default_check_only_reproduces_the_legacy_corpus_filter`.

**Note on the headline over-fire numbers.** `worst_fp_*` is a maximum over corpora,
not a mean. Both runs report 13.83% / 32.93%, carried entirely by `uos` (Pascal); the
next-worst corpus is fastapi at 1.46%. `uos` entered the panel with #147 and its
small-corpus limit was recorded then — it predates this change and is unaffected by it.

Raw dashboards: `benchmarks/results/baseline-0c734263/` and
`benchmarks/results/check-only-verify/`.
