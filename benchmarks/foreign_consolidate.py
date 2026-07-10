#!/usr/bin/env python3
"""Consolidate an `argot-bench --mode production` run into landing/src/data/foreign.json
— the per-signal scorecard for detector 1 (foreign-pattern detection, the gate).

Two halves, joined per corpus:
  CATCH  per gated class (foreign_import / foreign_api / foreign_concurrency), from the
         production run's per-fixture results (category → class, flagged → caught).
  FALSE-ALARM  over-fire vs novel-pattern detection, existing- vs new-file, from the
         committed CI dashboard (landing/src/data/benchmarks/latest.json, the leak-free
         #92 temporal-holdout numbers). Over-fire is the true false alarm.

Run the production sweep first:
  ./target/release/argot-bench --mode production --data-dir benchmarks/data \\
      --results-dir benchmarks/results/foreign-prod
  benchmarks/foreign_consolidate.py [prod_dir]     # default: benchmarks/results/foreign-prod
"""
import json, os, sys, glob
from collections import Counter, defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROD = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "benchmarks", "results", "foreign-prod")
LATEST = os.path.join(ROOT, "landing", "src", "data", "benchmarks", "latest.json")
OUT = os.path.join(ROOT, "landing", "src", "data", "foreign.json")
LL = {"python": "Python", "typescript": "TypeScript", "javascript": "JavaScript",
      "go": "Go", "rust": "Rust", "ruby": "Ruby", "c": "C", "cpp": "C++",
      "java": "Java", "csharp": "C#", "php": "PHP", "multi": "Python"}
# Two gated foreign capabilities. `foreign_concurrency` was folded into
# `foreign_import` (a foreign concurrency lib is just a foreign dep, caught by the
# same import stage — its own column was a redundant flat ~100%; see the evidence).
GATED = ["foreign_import", "foreign_api"]
SECONDARY = {"naming_shape_break", "semantic_convention"}


# The gated-vs-secondary decision is NOT inferred here. Each fixture's canonical
# scoring class is authored in its manifest (`class:` — see benchmarks/catalogs/*/
# manifest.yaml, validated canonical by argot-bench at load) and written into every
# result by the Rust bench. This consolidation just reads it — no category-name
# heuristics, no corpus-specific lists. See docs/research/evidence/
# foreign-classification-decoupled.md.
_CLASS_MAP = None  # lazy {(corpus, fixture_id): canonical_class} fallback


def _class_map():
    """Manifest-authored (corpus, id) → canonical class. Lazy + yaml-only-on-demand:
    used only for result files written before the bench emitted the `class` field."""
    global _CLASS_MAP
    if _CLASS_MAP is None:
        import yaml  # dev/bench dependency; only needed for pre-refactor results
        _CLASS_MAP = {}
        cat_dir = os.path.join(ROOT, "benchmarks", "catalogs")
        for mf in glob.glob(os.path.join(cat_dir, "*", "manifest.yaml")):
            m = yaml.safe_load(open(mf))
            for fx in m.get("fixtures", []):
                _CLASS_MAP[(m["corpus"], fx["id"])] = fx.get("class") or fx.get("category")
    return _CLASS_MAP


def fixture_class(corpus, f):
    """Canonical scoring class of a result fixture: the `class` the bench wrote,
    else the manifest-authored class (never a name heuristic)."""
    return f.get("class") or _class_map().get((corpus, f["id"]))


def catch_by_class(prod_dir):
    out = {}
    for fn in sorted(glob.glob(f"{prod_dir}/production-*.json")):
        d = json.load(open(fn))
        by = defaultdict(lambda: {"caught": 0, "total": 0, "vis_c": 0, "vis_t": 0,
                                  "mask_c": 0, "mask_t": 0, "reasons": Counter()})
        for f in d.get("fixture_results", []):
            b = by[fixture_class(d["corpus"], f)]
            b["total"] += 1
            visible = (f.get("difficulty") or "").lower() in ("easy", "medium")
            b["vis_t" if visible else "mask_t"] += 1
            if f.get("flagged"):
                b["caught"] += 1
                b["vis_c" if visible else "mask_c"] += 1
                for r in f.get("reasons", []):
                    b["reasons"][r] += 1
        out[d["corpus"]] = by
    return out


def cell(b):
    if not b or not b["total"]:
        return None
    vt, mt = b["vis_t"], b["mask_t"]
    # PRIMARY catch = VISIBLE foreign (easy+medium: an explicit import / FQN call /
    # distinct API name — what an LLM introduces visibly). MASKED (hard: a foreign
    # call whose name collides with the repo's own, an attested root namespace, or a
    # dynamic import) is a documented statistical limit of a name-based guardrail,
    # reported separately, not the headline.
    return {"caught": b["vis_c"], "total": vt,
            "pct": round(100 * b["vis_c"] / vt) if vt else None,
            "masked_caught": b["mask_c"], "masked_total": mt,
            "masked_pct": round(100 * b["mask_c"] / mt) if mt else None,
            "overall_pct": round(100 * b["caught"] / b["total"]),
            "reason": b["reasons"].most_common(1)[0][0] if b["reasons"] else None}


def main():
    byc = catch_by_class(PROD)
    latest = json.load(open(LATEST))
    fp = {c["corpus"]: c for c in latest["corpora"]}

    def rate(x):
        return round(x["rate_pct"], 2) if x else None

    rows = []
    for c in sorted(set(byc) | set(fp)):
        cls = byc.get(c, {})
        f = fp.get(c, {})
        rows.append({
            "corpus": c, "language": LL.get(f.get("language"), f.get("language", "?")),
            "classes": {k: cell(cls.get(k)) for k in GATED},
            "secondary": {k: cell(v) for k, v in cls.items() if k not in GATED},
            "overfire_existing_pct": rate(f.get("fp_existing_overfire") or f.get("fp_existing")),
            "detection_existing_pct": rate(f.get("fp_existing_detection")),
            "overfire_new_pct": rate(f.get("fp_new_overfire") or f.get("fp_new_file")),
            "under_sampled": f.get("under_sampled", False),
        })
    json.dump({"generated_at": "2026-07-08", "corpora": rows}, open(OUT, "w"), indent=1)

    agg = defaultdict(lambda: [0, 0])
    for r in rows:
        for k, v in r["classes"].items():
            if v:
                agg[k][0] += v["caught"]; agg[k][1] += v["total"]
    print(f"wrote {OUT}: {len(rows)} corpora")
    for k in GATED:
        cc, tt = agg[k]
        print(f"  {k:22s} {cc}/{tt} = {round(100*cc/tt) if tt else 0}%")


if __name__ == "__main__":
    main()
