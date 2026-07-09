# Architecture-graph foreignness: a discrete, low-FP "has no place here" signal

**Date:** 2026-07-09 · **Branch:** `feat/semantic-layer` · status: **promising cheap-probe
signal — worth a full validation.** Harness: `benchmarks/arch_graph_probe.py` (Python `ast`,
7 corpora). Opened after the node-kind n-gram *shape* gate hit an irreducible floor
([`foreign-structure-gate-floor.md`](foreign-structure-gate-floor.md)).

## The idea

The base gate catches a foreign **dependency** (an external import 0-usage in the repo). This
catches a foreign **relationship**: an *internal* module-dependency edge the repo's own topology
never has — a layer it never crosses, or a dependency **direction** it never uses (a `models/`
file importing `views/`). Two properties make it attractive where the shape gate failed:

- **Discrete, high-information** — an edge either crosses a boundary / reverses a direction or it
  does not, exactly the property that makes the import gate ~98%. (Node-kind n-grams are
  continuous and low-information; that is why they diluted to an 8–13% ceiling.)
- **Invisible to the base gate** — the imported module is the repo's *own* code, so vocabulary
  detection sees nothing. Non-overlapping signal.

Domain-blind: "layer" = the path component under a package root (never a hardcoded layer name).
Internal edges only (external deps are the base gate's job).

## Method (cheap, no scorer change)

Per corpus: detect package roots (any dir with `__init__.py` whose parent has none; skip
test/example/infra trees), map every file to its layer, resolve each internal import to a target
layer, and build the weighted cross-layer edge graph. Then:

- **Q1 signal exists?** directional asymmetry `asym%` = share of cross-layer edge-mass in the
  dominant direction of each pair. High ⇒ a near-DAG layering to violate.
- **Q2 FP** (over-fire proxy, 70/30 file split): of held-out files' cross-layer edges, the share
  that are the **clean tell** — `reversal` (novel edge whose reverse is attested) ∪ `sink-out`
  (novel edge from a fit-graph *sink* layer: imported-but-never-imports). "Any novel edge"
  (`novel%`) is the noisy upper bound (organic growth adds new edges constantly).
- **catch** (coverage estimate): of cross-layer edges the repo does *not* have, the share the
  reversal∪sink rule would flag if an LLM created one (uniform-violation model).

## Result

| corpus | layers | x-edges | asym% | novel% | **FP%** (clean tell) | **catch%** (est.) |
|---|--:|--:|--:|--:|--:|--:|
| scrapy | 36 | 122 | 95% | 13.8% | 0.7% | 68% |
| rich | 70 | 69 | 100% | 14.7% | 0.0% | 100% |
| faker | 14 | 23 | 100% | 9.5% | 0.0% | 77% |
| fastapi | 32 | 50 | 98% | 35.7% | 0.0% | 67% |
| wagtail | 38 | 158 | 94% | 17.0% | 2.1% | 56% |
| saleor | 34 | 298 | 90% | 7.1% | 3.1% | 40% |
| dagster | 239 | 828 | 95% | 10.8% | 2.5% | 54% |
| **mean** | | | **95%** | 15.5% | **1.2%** | **66%** |

- **Signal exists, strongly:** module graphs are 90–100% directional — real layering to violate,
  unlike the node-kind alphabet which had no clean discrete tell.
- **FP is low and gatable:** the clean tell (reversal ∪ sink-out) over-fires **≤3.1% on every
  corpus** (mean 1.2%), well inside the ≤5% budget. Firing on *any* novel edge (`novel%` up to
  36%) would not be gatable — the direction/sink discrimination is what makes it clean.
- **Catch is high (estimated ~66%)** at that FP — a different universe from the shape gate's
  8–13%. Even the weakest corpus (saleor, a big flat app with looser layering) is 40% catch at
  3.1% FP.

## Honest limits (what the cheap probe does NOT yet show)

1. **Catch is a coverage model, not measured recall.** It assumes a violation is uniform over the
   repo's missing cross-layer edges; real LLM violations cluster on plausible-but-wrong targets.
   Needs authored/synthetic architectural-violation fixtures (the bench way) for a true recall.
2. **FP is a 70/30 file split, not a real temporal holdout.** The honest over-fire is on the
   repo's own future *commits*; needs the real fit-at-`HEAD~window` + replay measure (as built
   for the structural sense in `argot-bench --mode structural`).
3. **Python only.** Import resolution differs per language (JS/TS re-exports, Go packages, Java
   packages, C includes). Cross-language validation required before any claim of generality.
4. **Coverage gap by construction:** the rule catches reversal/sink violations; a *novel-forward*
   edge (a new edge in a legal-ish direction from a non-sink layer) is not cleanly gatable — that
   is the ceiling on catch, and why catch is ~66% not ~100%.

## Why this is the right lever (vs the closed shape gate)

The shape gate failed because node-kind n-grams are the weakest representation of foreignness —
continuous, low-information, and every foreign snippet is built from shapes the repo also uses.
The signal that code "has no place here" is **relational**: it lives in the *edges* between
modules, not the *shapes* within a function. A dependency-direction reversal is discrete and rare
in healthy code (≤3%), so it can gate — the same reason a foreign import can.

## Next (if pursued)

Mirror the structural validation but on the graph: (1) authored/synthetic architectural-violation
fixtures for a true catch number; (2) real temporal-holdout FP on clean commits; (3) cross-language
edge extraction (build on `import_graph.rs`); (4) one full bench. Only then decide gate vs
advisory. This memo records the cheap-probe green light, not a shipped result.

**Reproduce:** `source .venv/bin/activate && python benchmarks/arch_graph_probe.py`.
