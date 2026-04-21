# Phase 13 Recon: ast_structural Scorer — 2026-04-21

## What does the existing scorer do?

**File:** `engine/argot/research/signal/scorers/ast_structural.py`  
**Feature extractor:** `engine/argot/research/signal/ast_features.py`

### Feature definition

`extract_features()` walks every AST node and emits `(NodeClassName, dotted_name)` pairs,
where `dotted_name` is a resolved `Name` or `Attribute` chain (e.g. `"BackgroundTasks.add_task"`).

This is **not** a treelet. It captures "which dotted identifiers appear under which node types"
— vocabulary attached to structural positions, not pure structural shape.

Depth: effectively 1 — the node class is the parent, the dotted name is the leaf.
No child-to-child structural relationships are encoded.

### Is it contrastive?

**No.** It is purely one-sided.

It fits on a training corpus (git diff history), builds Laplace-smoothed frequency
tables per `(NodeClassName, dotted_name)` pair, then scores fixtures by how surprising
those pairs are relative to the corpus.

There is no reference corpus (Model B). High score = "rare in this repo's git history",
which conflates:
- Rare-in-repo-AND-rare-everywhere (genuinely weird) → should score high
- Rare-in-repo-BUT-common-everywhere (paradigm break, e.g. FastAPI idiom) → should score high
- Common-everywhere-AND-in-repo (idiomatic) → should score low

The scorer cannot distinguish these because it only sees the repo side.

### Why does it fire on only 4 of 51 fixtures?

Three compounding reasons:

1. **Identifier-based features embed naming conventions.** `BackgroundTasks.add_task`
   appears in the FastAPI corpus and therefore is NOT rare — so the scorer marks it
   as normal. Threading breaks (`threading.Thread`) are also common in general Python
   and appear in other repos, so they're not flagged either. The signal from
   vocabulary dissolves.

2. **Git diff corpus ≠ working tree.** The corpus is built from commit-diff hunks
   (the `hunk_tokens` field in records). FastAPI repos' git history contains a lot of
   framework-specific code, making `BackgroundTasks`, `Depends`, etc. appear frequently
   — exactly the wrong baseline for detecting "is this idiomatic?".

3. **Bimodal score distribution.** For most fixtures, the scorer emits scores near 0
   (because the features encountered are common in the repo diff history). Only when a
   fixture contains an unusual identifier chain that never appeared in training does
   it produce a non-zero score. This creates the bimodal distribution noted in Phase 12.

### What corpus does it use?

Git diff history. The `fit()` method is called with `corpus: list[dict[str, Any]]` records
where each record represents a commit diff hunk. This is the standard argot training corpus,
not the working tree.

---

## Verdict: Fundamentally different from Phase 13 design

The existing scorer shares surface overlap (uses AST, fits on corpus) but differs on
every key design axis:

| axis | ast_structural (existing) | Phase 13 design |
|---|---|---|
| Feature type | dotted identifier names under node types | structural treelets (node type + child types only) |
| Naming-invariant? | No — identifier strings embedded | Yes — type signatures only |
| Contrastive? | No — one-sided anomaly | Yes — log-ratio vs. generic reference |
| Model B | none | CPython stdlib frequency table |
| Model A corpus | git diff history | working-tree .py files |
| Depth | 1 (node + leaf identifier) | 1 AND 2 (parent + children, multi-granularity) |

**Conclusion: rewrite, do not extend.**

The Phase 13 scorer needs a new treelet extractor and a two-corpus contrastive
scoring function. Adapting `ast_structural.py` would require changing every design
choice — it is cleaner to implement fresh.

---

## Phase 13 implementation notes

Based on this diagnosis, the new scorer must:

1. Define treelets as `(parent_type, frozenset_of_child_types)` for depth 1
   and `(parent_type, child_type, frozenset_of_grandchild_types)` for depth 2.
   Only AST node type names — no `id`, `name`, or `attr` string values.

2. Build Model A by walking `*.py` files in the repo working tree (not git history).
   Cache keyed to HEAD commit SHA.

3. Build Model B once from CPython stdlib (depth 1 + depth 2 treelets combined).
   Store as `engine/argot/research/reference/generic_treelets.json`.

4. Score = `mean over hunk treelets t: log(count_A(t) + ε) - log(count_B(t) + ε)`.
   Normalize by treelet count. Score neutral (0.0) if < 3 treelets extracted.

5. Register as `ast_contrastive` in `REGISTRY`.
