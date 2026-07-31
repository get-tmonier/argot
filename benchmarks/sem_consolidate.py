#!/usr/bin/env python3
"""Consolidate a sem_all.py sweep (+ the committed 3-judge FP labels) into
landing/src/data/semantic.json — the data the benchmarks page renders.

Every row is UNIFORM and COMPLETE: each reinvention row carries recall (pct +
caught/total) AND raw clean-commit false-fire (fires + hunks + per-hunk%) AND, from
the adversarial labels, the judged-true rate; each misplacement row carries
transplant recall (+ sample size), in-place over-fire, and clean-commit misplaced FP.
A cell only goes '—' on a genuine measurement gap (a fit error, or a single-package
repo with no second area for placement) — never as a formatting shortcut.

Inputs (regenerate the first with `just bench-semantic`):
  benchmarks/results/sem_all_*.jsonl   one row per corpus (this sweep)
  benchmarks/semantic-labels/*.json    3-judge genuine/false-alarm verdicts (committed)

The fire sets are deterministic at a fixed window, so the labels' genuine-rate applies
to the sweep's raw fire count (exact when fully labelled, scaled when sampled — the row
is marked `fp_sampled`) — but ONLY within one embedding model. A different embedder
fires on different pairs, so a genuine-rate adjudicated against another model's fires
says nothing about this one's. Each label file therefore records the `model` it was
judged against; a label whose model differs from the sweep's is refused, the derived
columns go null, and the corpus is listed under `stale_labels` so the page shows a
measurement gap instead of a number carried over from a model that no longer exists.

Usage: benchmarks/sem_consolidate.py [--window N]
"""
import json, os, glob, subprocess, sys, datetime

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RESULTS = os.path.join(ROOT, "benchmarks", "results")
LABELS = os.path.join(ROOT, "benchmarks", "semantic-labels")
OUT = os.path.join(ROOT, "landing", "src", "data", "semantic.json")
LL = {"python": "Python", "typescript": "TypeScript", "javascript": "JavaScript",
      "go": "Go", "rust": "Rust", "ruby": "Ruby", "c": "C", "cpp": "C++",
      "java": "Java", "csharp": "C#", "php": "PHP", "pascal": "Object Pascal"}
WINDOW = 150
if "--window" in sys.argv:
    WINDOW = int(sys.argv[sys.argv.index("--window") + 1])


def pct(x):
    return round(100 * x) if x is not None else None


def per_hunk(fires, hunks):
    return round(100 * fires / hunks, 2) if (fires is not None and hunks) else None


def main():
    rows = {}
    for f in sorted(glob.glob(f"{RESULTS}/sem_all_*.jsonl")) + [f"{RESULTS}/sem_all.jsonl"]:
        if not os.path.exists(f):
            continue
        for line in open(f):
            line = line.strip()
            if line:
                d = json.loads(line)
                rows[d["corpus"]] = d  # last write wins (a re-run supersedes)

    sweep_model = next((r["model"] for r in rows.values() if r.get("model")), None)
    labels, stale_labels = {}, []
    for f in glob.glob(f"{LABELS}/*.json"):
        try:
            d = json.load(open(f))
        except Exception:
            continue
        # Fails CLOSED: an unstamped sweep cannot prove the labels match, and
        # "we could not check" must not read the same as "it matched".
        if d.get("model") != sweep_model or sweep_model is None:
            stale_labels.append(d["corpus"])
            continue
        labels[d["corpus"]] = d
    if stale_labels:
        why = ("the sweep records no model" if sweep_model is None
               else "judged against another model")
        print(f"  ! labels refused ({why}): {', '.join(sorted(stale_labels))}",
              file=sys.stderr)

    f1, f2 = [], []
    for c in sorted(rows):
        d = rows[c]
        lang = d.get("language", "?")
        f1d, f2d, fpd = d.get("f1") or {}, d.get("f2") or {}, d.get("fp") or {}
        errs = d.get("errors", [])
        h = fpd.get("hunks_scanned")
        red, mis = fpd.get("redundant_fp"), fpd.get("misplaced_fp")

        # judged true-FP: the labelled sample's genuine-rate applied to this run's raw.
        lab = labels.get(c, {})
        n_lab = lab.get("raw_fires")
        fp_true = fp_genuine = fp_true_ph = None
        sampled = False
        if red == 0:
            fp_true = fp_genuine = 0
            fp_true_ph = 0.0
        elif n_lab and red is not None:
            gen_rate = lab["genuine"] / n_lab
            fp_true = round(red * (1 - gen_rate))
            fp_genuine = red - fp_true
            fp_true_ph = per_hunk(fp_true, h)
            sampled = n_lab < red

        f1.append({
            "corpus": c, "language": LL.get(lang, lang),
            "recall_pct": pct(f1d.get("recall")),
            "recall_caught": f1d.get("fired"), "recall_total": f1d.get("planted"),
            "fp_fires": red, "fp_hunks": h, "fp_per_hunk_pct": per_hunk(red, h),
            "fp_true": fp_true, "fp_true_per_hunk_pct": fp_true_ph,
            "fp_genuine": fp_genuine, "fp_labeled": n_lab, "fp_sampled": sampled,
            "errors": errs,
        })
        of = f2d.get("overfire")  # fraction 0-1 (LOO in-place fire rate)
        f2.append({
            "corpus": c, "language": LL.get(lang, lang),
            "recall_pct": pct(f2d.get("recall")), "recall_eval": f2d.get("place_eval"),
            "overfire_pct": round(100 * of, 2) if of is not None else None,
            "fp_fires": mis, "fp_hunks": h, "fp_per_hunk_pct": per_hunk(mis, h),
            "errors": errs,
        })

    commit = subprocess.run(["git", "-C", ROOT, "rev-parse", "--short", "HEAD"],
                            capture_output=True, text=True).stdout.strip()
    doc = {"generated_at": datetime.date.today().isoformat(), "commit": commit,
           "window": WINDOW, "model": sweep_model,
           "stale_labels": sorted(stale_labels),
           "reinvention": f1, "misplacement": f2}
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    json.dump(doc, open(OUT, "w"), indent=1)

    recs = [x["recall_pct"] for x in f1 if x["recall_pct"] is not None]
    f2r = [x["recall_pct"] for x in f2 if x["recall_pct"] is not None]
    tf = sum(x["fp_fires"] or 0 for x in f1)
    tg = sum(x["fp_genuine"] or 0 for x in f1)
    th = sum(x["fp_hunks"] or 0 for x in f1)
    print(f"wrote {OUT}: {len(f1)} corpora (window {WINDOW}, commit {commit})")
    if recs:
        print(f"  F1 recall min {min(recs)} med {sorted(recs)[len(recs)//2]}; "
              f"{tf} fires, {tg} genuine ({round(100*tg/tf)}%), raw {round(100*tf/th,2)}%/hunk")
    if f2r:
        print(f"  F2 recall min {min(f2r)} med {sorted(f2r)[len(f2r)//2]}; "
              f"unmeasured (single-area): {[x['corpus'] for x in f2 if x['recall_pct'] is None]}")
    miss = [(x["corpus"], x["errors"]) for x in f1 if x["errors"]]
    if miss:
        print(f"  corpora with errors: {miss}")


if __name__ == "__main__":
    main()
