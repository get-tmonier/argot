# Issue #92 step ③ — AST idiom-surprisal scout: confounded, no clean lever

**Date:** 2026-07-03 · **Branch:** `bench/92-temporal-holdout` · Scout:
`crates/argot-bench/src/bin/idiom_scout.rs` (removed after this run). Outcome:
**no clean signal above the ~0.65 plateau**; the promising numbers were the
authored-novelty confound the prior scouts documented.

## Hypothesis

The in-vocabulary hard classes are invisible to *subword* BPE surprise (`die`
splits into common subwords, Δsurprise≈0.00) but might separate as **AST
fingerprint** surprise — a construct/subtree the repo never uses. Four
fingerprint families over the repo's own AST (document-frequency), max
`-ln(df/n)` per hunk: `K` node-kind, `PK` parent>kind, `T` kind:leaf-text,
`PT` parent>kind:text. AUC(break vs control) per category, laravel + redis.

## Result

| control | K | PK | T | PT | tell |
|---|---:|---:|---:|---:|---|
| old repo windows | 0.60 | 0.62 | **1.00** | **1.00** | uniform 1.00 across *all* classes |
| real git-diff blocks | 0.09 | 0.04 | 0.96 | 0.70 | `T`/`PT` still uniform across classes; `K` inverted |

Both controls are confounded:
- **`T`/`PT` uniform across classes** — identical AUC for lexically-obvious
  `foreign_import` and subtle `wrong_error_discipline` is the signature of an
  *authored-vs-real* / novel-identifier detector, not a convention detector.
  Whole-identifier fingerprinting *does* fire on `die` (df=0) where BPE-subwords
  did not — but it fires on **any** fresh identifier (every new local variable),
  so its max-surprise saturates on both breaks and legitimate new code → high FP.
- **`K`/`PK` inverted on the diff control** — raw added-line blocks parse as
  broken fragments (ERROR nodes score as rare kinds), so structural node-kind
  surprise is noise. The clean-window `K`≈0.60 is weak anyway.

## The one real (but bounded) sub-lever

The signal `die` produces — `t:name:die`, df=0 as a *whole* identifier — is
genuine and is exactly why `die` is missed today: tree-sitter parses `die(…)`
as a construct/statement, not a `call`, so `call_receiver`'s callee attestation
skips it. Extending attestation to **language-construct keywords** (`die`,
`exit`, `goto`…) the repo never uses would recover the *wholly-absent-construct*
subset (`die`, `curl_init`, `$_GET`, `pcntl_fork`) — perhaps 2–4 fixtures per
corpus — at low FP (a small builtin vocabulary, unlike free identifiers). It
does **not** touch the argument-value class (`E_USER_ERROR` where
`trigger_error` is attested; `malloc` where `malloc` is attested), and a curated
per-language construct list brushes the no-hardcoded-domain rule.

## Verdict

Idiom-surprisal is not a general lever — it reproduces the confound wall of the
7 prior methods (JEPA, joint/per-token MLM, CodeRankEmbed, manifold outlier, all
≤0.65 when fairly controlled). No local, no-runtime method clears ≥85% on the
in-vocabulary hard classes. The honest path is to re-scope the rubric to what
argot's mechanism addresses (lexical/structural voice) and adopt a
precision-at-coverage metric (Naturalize), reporting the argument-value semantic
class as an out-of-scope known limit rather than gating on it.
