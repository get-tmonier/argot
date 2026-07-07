#!/usr/bin/env python3
"""Index-based half of the semantic bench: the numbers computable from a corpus's
production `.argot/semantic-index.json` alone (no authored fixtures) —

  reinvention OVER-FIRE  : leave-one-out, fraction of the repo's own functions
                            whose cross-file margin exceeds the calibrated bar
                            (= "fires on the repo's own existing code").
  placement RECALL        : synthetic transplant (each fn re-filed into a random
                            foreign area) that fires misplaced.
  placement OVER-FIRE     : in-place functions that fire misplaced.

These use the EXACT production thresholds (baked into the artifact + the scorer
constants) so they match the shipped binary. Reinvention RECALL is measured
separately by sem_bench.py (real CLI). Usage: sem_analysis.py <corpus_repo>
"""
import json, base64, struct, math, sys, random, os
from collections import Counter

random.seed(0)
IDX = sys.argv[1].rstrip("/") + "/.argot/semantic-index.json"
NAME = sys.argv[1].rstrip("/").split("/")[-1]

# production constants (mirror crates/argot-core/src/scoring/semantic/*)
AREA_DEPTH = 3
K = 10
MIN_NEIGH = 5
MISPLACED_FACTOR = 0.3
ABS_CEILING = 0.05
MIN_TOP2 = 0.8
DEFAULT_NORM = 0.3
# reinvention fire rule (mirror redundant.rs)
CONFIRM_SIM = 0.85
CALLEE_BAR = 0.15
MIN_CALLEES = 2


def area_of(p, depth=AREA_DEPTH):
    c = p.split("/")
    return "/".join(c[:-1][:depth]) if len(c) > 1 else ""


def load(path):
    d = json.load(open(path))["languages"]
    lang = next(iter(d))  # python or typescript
    d = d[lang]
    raw = base64.b64decode(d["vectors_b64"])
    dim, cnt = d["dim"], d["count"]
    flat = struct.unpack("<%de" % (cnt * dim), raw)

    def norm(v):
        n = math.sqrt(sum(x * x for x in v)) or 1.0
        return [x / n for x in v]

    vecs = [norm(list(flat[i * dim:(i + 1) * dim])) for i in range(cnt)]
    callees = [set(c) for c in d.get("callees", [[]] * cnt)]
    return vecs, d["paths"], d.get("margin_bar", 0.0), d.get("area_norms", {}), lang, callees, d["symbols"]


def cos(a, b):
    return sum(x * y for x, y in zip(a, b))


def main():
    vecs, paths, bar, norms, lang, callees, syms = load(IDX)
    n = len(vecs)
    step = max(1, n // 500)
    sample = list(range(0, n, step))

    def jac(a, b):
        return len(a & b) / len(a | b) if (a or b) else 0.0

    # --- reinvention over-fire: LOO, the production callee-confirm OR margin rule ---
    # Applies the production same-name (move) gate: a nearest match with the same
    # symbol is a move/rename, not a reinvention → no fire (production returns None).
    pairs = []  # (c1, c2, callee_jac, cand_callees, match_callees)
    for qi in sample:
        sims = sorted(((cos(vecs[qi], vecs[j]), j) for j in range(n)
                       if paths[j] != paths[qi]), reverse=True)[:2]
        if len(sims) < 2:
            continue
        (c1, bi), (c2, _) = sims[0], sims[1]
        gated = syms[qi].lower() == syms[bi].lower()  # move gate → never fires
        pairs.append((c1, c2, jac(callees[qi], callees[bi]), len(callees[qi]), len(callees[bi]), gated))

    def of_rule(sim, cbar, use_margin):
        f = 0
        for c1, c2, cj, la, lb, gated in pairs:
            if gated:
                continue
            mf = use_margin and c1 >= 0.80 and (c1 - c2) > bar
            cf = c1 >= sim and la >= MIN_CALLEES and lb >= MIN_CALLEES and cj >= cbar
            if mf or cf:
                f += 1
        return f / len(pairs) if pairs else 0.0

    if os.environ.get("SWEEP"):
        print(f"\n[{NAME}] reinvention over-fire sweep (index callees):")
        for sim in (0.85, 0.88):
            for cbar in (0.15, 0.20, 0.25, 0.30, 0.40):
                print(f"   sim>{sim} callee>{cbar:.2f} +margin: {of_rule(sim,cbar,True):.1%}   callee-only: {of_rule(sim,cbar,False):.1%}")
    reinv_of = of_rule(CONFIRM_SIM, CALLEE_BAR, True)

    # --- placement: synthetic transplant recall + in-place over-fire ---
    areas = sorted({area_of(p) for p in paths})
    place_recall_fire = place_of_fire = place_eval = 0
    if len(areas) >= 2:
        for qi in sample:
            sims = sorted(((cos(vecs[qi], vecs[j]), j) for j in range(n) if j != qi),
                          reverse=True)[:K]
            nb = [area_of(paths[sims[k][1]]) for k in range(len(sims))]
            if len(nb) < MIN_NEIGH:
                continue
            place_eval += 1
            actual = area_of(paths[qi])

            def fires(claimed):
                in_area = sum(1 for a in nb if a == claimed) / len(nb)
                counts = Counter(nb).most_common()
                modal = counts[0][0]
                top2 = sum(c for _, c in counts[:2]) / len(nb)
                if modal == claimed:
                    return False
                norm = norms.get(claimed, DEFAULT_NORM)
                return in_area <= ABS_CEILING and in_area < norm * MISPLACED_FACTOR and top2 >= MIN_TOP2

            if fires(actual):            # in-place fires → over-fire
                place_of_fire += 1
            foreign = random.choice([a for a in areas if a != actual])
            if fires(foreign):           # transplanted fires → recall
                place_recall_fire += 1

    out = {
        "corpus": NAME, "language": lang, "functions": n,
        "reinvention_over_fire": round(reinv_of, 4),
        "placement_recall": round(place_recall_fire / place_eval, 4) if place_eval else None,
        "placement_over_fire": round(place_of_fire / place_eval, 4) if place_eval else None,
        "n_areas": len(areas),
    }
    print(f"\n== {NAME} ({lang}, {n} fns, {len(areas)} areas) ==")
    print(f"  reinvention over-fire (LOO): {reinv_of:.1%}")
    if place_eval:
        print(f"  placement recall (transplant): {place_recall_fire/place_eval:.1%}   over-fire (in-place): {place_of_fire/place_eval:.1%}")
    else:
        print("  placement: N/A (<2 areas)")
    print(json.dumps(out))


if __name__ == "__main__":
    main()
