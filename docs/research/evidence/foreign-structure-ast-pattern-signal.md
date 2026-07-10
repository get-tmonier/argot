# Foreign-structure detection: AST patterns as vocabulary (first signal)

**Date:** 2026-07-09 · **Branch:** `feat/semantic-layer` · status: **exploratory — signal
confirmed, not yet built.** Harness: `benchmarks/struct_ngram_probe.py`.

## Question

argot's gate catches foreign **vocabulary** — an import/callee 0-usage in the repo. But
"foreign to the repo's voice" is broader: an LLM can paste a whole function written in a
structurally-foreign *style* using only the repo's own vocabulary (`for` vs a
comprehension, lambda vs named fn, guard-clauses vs nested `else`, raw string-SQL vs the
ORM). Can argot catch **structural** foreignness — and at a gatable false-alarm rate?

This surfaced from the foreign_api "gap" audit: dagster/outline's residual misses were
either invalid fixtures (react-router is imported 107× in dagster — the fixture was
wrong) or **no-foreign-symbol structural breaks** (`<style jsx>`, an express-shaped
handler, string SQL) that vocabulary detection cannot see by construction.

## Method (cheap novelty-separation harness — no scorer change)

Domain-blind features only: tree-sitter/`ast` **node-kind** shape — never identifiers,
strings, or framework literals (matches `shape_primitive.rs`'s design + the
no-hardcoded-domain rule). "Foreign-style" proxy = functions from a *different*
same-language repo (the realistic "generically-styled paste" case). 6 Python corpora
(scrapy 1096 fns · rich 604 · faker 563 · fastapi 523 · saleor 4006 · wagtail 2522).
For each native repo: fit on 70% of its fns, score its held-out 30% (should read
*typical*) vs every foreign repo's fns (should read *foreign*). Report AUC and catch at a
threshold set to a 5%/1% false-alarm budget on the native held-out.

Two formulations compared:
1. **Aggregate per-function features** — a ~23-dim vector of structural-idiom rates
   (loop/comp/lambda/ternary/try/nesting/…), Mahalanobis novelty.
2. **AST-patterns-as-vocabulary** — the argot-native framing: node-kind n-grams
   (bigram parent→child, trigram gp→p→child) are the repo's *structural vocabulary*;
   foreignness = fraction of a hunk's n-grams **0-usage** in the repo (attested =
   appears in ≥2 train fns). This mirrors import/callee 0-usage detection, on shape.

## Result

| formulation | mean AUC | catch @5% FP | catch @1% FP |
|---|---|---|---|
| aggregate per-fn features (Mahalanobis novelty) | 0.60 | 17% | 9% |
| **0-usage AST bigram patterns** | **0.74** | 26% | 12% |
| **0-usage AST trigram patterns** | **0.72** | 28% | 12% |

**The vocabulary framing clearly wins** (0.74 vs 0.60) — flagging the genuinely-unseen
beats "is this atypical," the same low-FP property that makes import detection work.

**Separation scales with codebase size — the signature of a real signal, not an
artifact:** as the native voice,
- saleor (2.6k files) → foreign AUC **0.79–0.95**; wagtail (821) → **0.67–0.92**;
- small repos scrapy/rich → **0.55–0.65**.

A mature codebase has a rich structural vocabulary, so a pasted foreign pattern stands
out; a young one lacks the voice yet — the same "a voice needs a corpus" law as the base
gate.

## Honest limits

- **Gatable-recall not yet there.** At a usable FP budget catch is modest (~27%/5%FP,
  ~12%/1%FP). Today this is an **advisory / ranking** signal (AUC 0.72–0.74 surfaces the
  most-foreign code well), strong on mature repos (0.85–0.95), weak on small ones — not
  yet a high-recall gate like import detection (~98%).
- **Cross-repo conflates some domain with style.** "Django-shaped code in scrapy" *is*
  foreign-to-this-repo, so it's a fair proxy — but the cleanest style-not-domain test
  (same repo, functions mechanically restyled) is unbuilt and is the next harness.
- **Distinct from the settled limit.** This is 0-usage-*pattern* detection, NOT the
  net-negative single-construct "convention" style (`die`/`throw`; +1 catch/+45 FP —
  see the issue-92 capstone). Different mechanism; do not conflate.

## Next (see the driving goal / `struct_ngram_probe.py`)

Refine on the cheap harness before any scorer change: IDF/rarity-weight patterns
(mirror the subtoken-IDF signal); richer patterns (depth-4 paths, subtree productions,
control-flow motifs); fire on one strong-foreign pattern vs a fraction; build the
same-repo-restyled test. Only once separation clears ~≥85%/≤5%, port into `argot-core`
on top of `shape_primitive.rs` and add a clean-commit temporal-holdout over-fire measure;
full bench once, at the end. Target ≥85/≤5 on every corpus, or an evidence-backed
per-corpus floor.
