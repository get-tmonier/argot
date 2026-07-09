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
GATED = ["foreign_import", "foreign_api", "foreign_concurrency"]
SECONDARY = {"naming_shape_break", "semantic_convention"}


def norm_class(cat):
    """Canonical class for a fixture category string (catalogs use many specific
    break types that roll up into the three gated classes + two secondary ones)."""
    c = (cat or "").lower()
    # In-vocabulary MISUSE of a library the repo already uses (calling a known
    # library's wrong/less-common method) is the ungated semantic_convention class
    # — argot deliberately does not gate it (it is not a *foreign* dependency/API,
    # and it is not the danger an LLM poses). Route it out of foreign_api.
    if "wrong_api_within_known" in c:
        return "semantic_convention"
    for k in set(GATED) | SECONDARY:
        if k in c:
            return k
    if "import" in c:
        return "foreign_import"
    if "concurren" in c or "async" in c or "thread" in c or "schedul" in c or "sleep" in c:
        return "foreign_concurrency"
    if "naming" in c or "shape" in c:
        return "naming_shape_break"
    if "error" in c or "discipline" in c or "convention" in c:
        return "semantic_convention"
    # everything else that is a foreign dependency/API call
    return "foreign_api"


def catch_by_class(prod_dir):
    out = {}
    for fn in sorted(glob.glob(f"{prod_dir}/production-*.json")):
        d = json.load(open(fn))
        by = defaultdict(lambda: {"caught": 0, "total": 0, "vis_c": 0, "vis_t": 0,
                                  "mask_c": 0, "mask_t": 0, "reasons": Counter()})
        for f in d.get("fixture_results", []):
            b = by[norm_class(f.get("category"))]
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
