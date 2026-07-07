#!/usr/bin/env python3
"""Index-based half of the semantic bench: the numbers computable from a corpus's
production `.argot/semantic-index.json` alone (no authored fixtures) —

  reinvention OVER-FIRE  : leave-one-out, fraction of the repo's own functions
                            that fire `redundant` against another existing
                            function (= "fires on the repo's own existing code").
  placement RECALL        : synthetic transplant (each fn re-filed into a random
                            foreign area) that fires misplaced.
  placement OVER-FIRE     : in-place functions that fire misplaced.

These mirror the EXACT production thresholds (the scorer constants) so they match
the shipped binary — the reinvention rule reads the same Rust-extracted callees +
subtokens the scorer confirms against. Reinvention RECALL is measured separately
by sem_bench.py (real CLI). Usage: sem_analysis.py <corpus_repo>
"""
import json, base64, struct, math, sys, random, os
from collections import Counter

random.seed(0)
IDX = sys.argv[1].rstrip("/") + "/.argot/semantic-index.json"
NAME = sys.argv[1].rstrip("/").split("/")[-1]

# placement constants (mirror crates/argot-core/src/scoring/semantic/placement.rs)
AREA_DEPTH = 3
K = 10
MIN_NEIGH = 5
MISPLACED_FACTOR = 0.3
ABS_CEILING = 0.05
MIN_TOP2 = 0.8
DEFAULT_NORM = 0.3
# reinvention two-tier fire rule (mirror redundant.rs)
NORMAL_SIM = 0.78
NORMAL_SUB = 0.40
NORMAL_CALLEE = 0.12
NORMAL_MIN_CALLEES = 2
STRONG_SIM = 0.70
STRONG_SUB = 0.52
STRONG_CALLEE = 0.30
STRONG_MIN_CALLEES = 3


def area_of(p, depth=AREA_DEPTH):
    c = p.split("/")
    return "/".join(c[:-1][:depth]) if len(c) > 1 else ""


def is_dunder(sym):  # mirror redundant.rs::is_dunder
    return len(sym) >= 5 and sym.startswith("__") and sym.endswith("__")


def is_test_path(path):  # mirror redundant.rs::is_test_path
    for comp in path.split("/"):
        c = comp.lower()
        if c in ("test", "tests", "spec", "specs", "__tests__"):
            return True
        stem = c.split(".")[0]
        if stem == "conftest" or stem.startswith("test_") or stem.endswith("_test") \
           or ".test." in c or ".spec." in c:
            return True
    return False


def is_reinvention_candidate(sym, path):
    return not is_dunder(sym) and not is_test_path(path)


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
    subtoks = [set(s) for s in d.get("subtokens", [[]] * cnt)]
    return vecs, d["paths"], d.get("area_norms", {}), lang, callees, subtoks, d["symbols"]


def cos(a, b):
    return sum(x * y for x, y in zip(a, b))


def main():
    vecs, paths, norms, lang, callees, subtoks, syms = load(IDX)
    n = len(vecs)
    step = max(1, n // 500)
    sample = list(range(0, n, step))

    def jac(a, b):
        return len(a & b) / len(a | b) if (a or b) else 0.0

    # --- corpus subtoken IDF (mirror RedundantScorer::new) ---
    df = Counter()
    for s in subtoks:
        for t in s:
            df[t] += 1
    N = max(1, n)

    def idf(t):
        return math.log((N + 1) / (df.get(t, 0) + 1)) + 1.0

    def wsub(a, b):
        union = a | b
        if not union:
            return 0.0
        wu = sum(idf(t) for t in union)
        wi = sum(idf(t) for t in (a & b))
        return wi / wu if wu else 0.0

    def fires(candi, matchi):
        """Two-tier production rule between candidate index cand vs match."""
        c = cos(vecs[candi], vecs[matchi])
        cj = jac(callees[candi], callees[matchi])
        sj = wsub(subtoks[candi], subtoks[matchi])
        both = lambda m: len(callees[candi]) >= m and len(callees[matchi]) >= m
        normal = c >= NORMAL_SIM and ((both(NORMAL_MIN_CALLEES) and cj >= NORMAL_CALLEE) or sj >= NORMAL_SUB)
        strong = c >= STRONG_SIM and ((both(STRONG_MIN_CALLEES) and cj >= STRONG_CALLEE) or sj >= STRONG_SUB)
        return normal or strong

    # --- reinvention over-fire: LOO, nearest cross-file, all production gates ---
    reinv_fire = reinv_eval = 0
    for qi in sample:
        if not is_reinvention_candidate(syms[qi], paths[qi]):  # dunder / test gate
            continue
        sims = sorted(((cos(vecs[qi], vecs[j]), j) for j in range(n)
                       if paths[j] != paths[qi]), reverse=True)[:1]
        if not sims:
            continue
        bi = sims[0][1]
        reinv_eval += 1
        if syms[qi].lower() == syms[bi].lower():  # move gate → never fires
            continue
        if fires(qi, bi):
            reinv_fire += 1
    reinv_of = reinv_fire / reinv_eval if reinv_eval else 0.0

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

            def place_fires(claimed):
                in_area = sum(1 for a in nb if a == claimed) / len(nb)
                counts = Counter(nb).most_common()
                modal = counts[0][0]
                top2 = sum(c for _, c in counts[:2]) / len(nb)
                if modal == claimed:
                    return False
                nrm = norms.get(claimed, DEFAULT_NORM)
                return in_area <= ABS_CEILING and in_area < nrm * MISPLACED_FACTOR and top2 >= MIN_TOP2

            if place_fires(actual):            # in-place fires → over-fire
                place_of_fire += 1
            foreign = random.choice([a for a in areas if a != actual])
            if place_fires(foreign):           # transplanted fires → recall
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
