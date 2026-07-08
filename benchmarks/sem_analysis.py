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
by sem_bench.py (real CLI); clean-commit misplaced/redundant FP by sem_fp.py.

Prod indexes are PER-LANGUAGE (check evaluates a function against only its own
language's index), so multi-language corpora (dagster) are analysed one language
at a time and aggregated by eval count. Usage: sem_analysis.py <corpus_repo> [--out path]
"""
import json, base64, struct, math, sys, random, os
from collections import Counter

random.seed(0)
REPO = sys.argv[1].rstrip("/")
IDX = REPO + "/.argot/semantic-index.json"
NAME = os.path.basename(os.path.dirname(REPO)) if REPO.endswith("/.repo") else os.path.basename(REPO)
OUT = None
for i, a in enumerate(sys.argv[2:]):
    if a.startswith("--out"):
        OUT = a.split("=", 1)[1] if "=" in a else sys.argv[2:][i + 1]

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


def parent_dir(p):
    return p.rsplit("/", 1)[0] if "/" in p else ""


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


def load_langs(path):
    """Yield (lang, vecs, paths, area_norms, callees, subtoks, symbols) per language."""
    d = json.load(open(path))["languages"]
    for lang, ld in d.items():
        raw = base64.b64decode(ld["vectors_b64"])
        dim, cnt = ld["dim"], ld["count"]
        if cnt == 0:
            continue
        flat = struct.unpack("<%de" % (cnt * dim), raw)

        def norm(v):
            n = math.sqrt(sum(x * x for x in v)) or 1.0
            return [x / n for x in v]

        vecs = [norm(list(flat[i * dim:(i + 1) * dim])) for i in range(cnt)]
        callees = [set(c) for c in ld.get("callees", [[]] * cnt)]
        subtoks = [set(s) for s in ld.get("subtokens", [[]] * cnt)]
        yield lang, vecs, ld["paths"], ld.get("area_norms", {}), callees, subtoks, ld["symbols"]


def cos(a, b):
    return sum(x * y for x, y in zip(a, b))


def analyse_lang(vecs, paths, norms, callees, subtoks, syms):
    n = len(vecs)
    step = max(1, n // 500)
    sample = list(range(0, n, step))
    index_dirs = {parent_dir(p) for p in paths}

    def jac(a, b):
        return len(a & b) / len(a | b) if (a or b) else 0.0

    sdf = Counter(); cdf = Counter()
    for s in subtoks:
        for t in s:
            sdf[t] += 1
    for cs in callees:
        for t in cs:
            cdf[t] += 1
    N = max(1, n)
    rare_df = max(4, math.ceil(0.004 * N))

    def wsub(a, b):
        union = a | b
        if not union:
            return 0.0
        w = lambda t: math.log((N + 1) / (sdf.get(t, 0) + 1)) + 1.0
        wu = sum(w(t) for t in union)
        wi = sum(w(t) for t in (a & b))
        return wi / wu if wu else 0.0

    def fires(candi, matchi):
        if syms[matchi] in callees[candi] or syms[candi] in callees[matchi]:
            return False
        c = cos(vecs[candi], vecs[matchi])
        cj = jac(callees[candi], callees[matchi])
        sj = wsub(subtoks[candi], subtoks[matchi])
        both = lambda m: len(callees[candi]) >= m and len(callees[matchi]) >= m
        normal = c >= NORMAL_SIM and ((both(NORMAL_MIN_CALLEES) and cj >= NORMAL_CALLEE) or sj >= NORMAL_SUB)
        strong = c >= STRONG_SIM and ((both(STRONG_MIN_CALLEES) and cj >= STRONG_CALLEE) or sj >= STRONG_SUB)
        rare = c >= STRONG_SIM and any(cdf.get(t, 0) <= rare_df for t in (callees[candi] & callees[matchi]))
        return normal or strong or rare

    reinv_fire = reinv_eval = 0
    for qi in sample:
        if not is_reinvention_candidate(syms[qi], paths[qi]):
            continue
        sims = sorted(((cos(vecs[qi], vecs[j]), j) for j in range(n)
                       if paths[j] != paths[qi]), reverse=True)[:1]
        if not sims:
            continue
        bi = sims[0][1]
        reinv_eval += 1
        if syms[qi].lower() == syms[bi].lower():
            continue
        if fires(qi, bi):
            reinv_fire += 1

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

            def place_fires(claimed, claimed_dir):
                # prod gate: placement can only judge a location with precedent.
                if claimed_dir not in index_dirs:
                    return False
                in_area = sum(1 for a in nb if a == claimed) / len(nb)
                counts = Counter(nb).most_common()
                modal = counts[0][0]
                top2 = sum(c for _, c in counts[:2]) / len(nb)
                if modal == claimed:
                    return False
                nrm = norms.get(claimed, DEFAULT_NORM)
                return in_area <= ABS_CEILING and in_area < nrm * MISPLACED_FACTOR and top2 >= MIN_TOP2

            if place_fires(actual, parent_dir(paths[qi])):  # in-place → over-fire
                place_of_fire += 1
            # transplant into a random foreign area, reusing a real dir of that area
            # so the prod new-dir gate is satisfied (relocation, not a new dir).
            foreign_areas = [a for a in areas if a != actual]
            foreign = random.choice(foreign_areas)
            foreign_dir = next((parent_dir(p) for p in paths if area_of(p) == foreign), foreign)
            if place_fires(foreign, foreign_dir):           # transplanted → recall
                place_recall_fire += 1

    return {"functions": n, "reinv_fire": reinv_fire, "reinv_eval": reinv_eval,
            "place_recall_fire": place_recall_fire, "place_of_fire": place_of_fire,
            "place_eval": place_eval, "n_areas": len(areas)}


def main():
    if not os.path.exists(IDX):
        print(f"no index at {IDX}", file=sys.stderr); sys.exit(1)
    agg = Counter()
    langs = []
    for lang, vecs, paths, norms, callees, subtoks, syms in load_langs(IDX):
        r = analyse_lang(vecs, paths, norms, callees, subtoks, syms)
        langs.append(lang)
        for k in ("functions", "reinv_fire", "reinv_eval", "place_recall_fire",
                  "place_of_fire", "place_eval"):
            agg[k] += r[k]
        agg["n_areas"] = max(agg["n_areas"], r["n_areas"])

    reinv_of = agg["reinv_fire"] / agg["reinv_eval"] if agg["reinv_eval"] else 0.0
    p_recall = agg["place_recall_fire"] / agg["place_eval"] if agg["place_eval"] else None
    p_of = agg["place_of_fire"] / agg["place_eval"] if agg["place_eval"] else None
    out = {
        "corpus": NAME, "languages": langs, "functions": agg["functions"],
        "reinvention_over_fire": round(reinv_of, 4),
        "placement_recall": round(p_recall, 4) if p_recall is not None else None,
        "placement_over_fire": round(p_of, 4) if p_of is not None else None,
        "place_eval": agg["place_eval"], "n_areas": agg["n_areas"],
    }
    print(f"\n== {NAME} ({'+'.join(langs)}, {agg['functions']} fns) ==")
    print(f"  reinvention over-fire (LOO): {reinv_of:.1%}")
    if p_recall is not None:
        print(f"  placement recall (transplant): {p_recall:.1%}   over-fire (in-place): {p_of:.1%}  (eval {agg['place_eval']})")
    else:
        print("  placement: N/A (<2 areas)")
    print(json.dumps(out))
    if OUT:
        os.makedirs(os.path.dirname(OUT) or ".", exist_ok=True)
        with open(OUT, "a") as fh:
            fh.write(json.dumps(out) + "\n")


if __name__ == "__main__":
    main()
