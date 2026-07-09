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
import numpy as np

random.seed(0)
REPO = sys.argv[1].rstrip("/")
IDX = REPO + "/.argot/semantic-index.json"
NAME = os.path.basename(os.path.dirname(REPO)) if REPO.endswith("/.repo") else os.path.basename(REPO)
OUT = None
for i, a in enumerate(sys.argv[2:]):
    if a.startswith("--out"):
        OUT = a.split("=", 1)[1] if "=" in a else sys.argv[2:][i + 1]

# placement v2 (mirror crates/argot-core/src/scoring/semantic/placement.rs):
# adaptive areas + entangled merges + (k, z) come SELF-CALIBRATED from the
# artifact's per-language `placement` block; the fire rule is the k-NN area
# vote (modal != claimed AND own-area count <= z). A disabled block = the repo
# abstains (placement N/A).
MIN_NEIGH = 5
MAX_CONTAINER_FRAC = 0.5
MIN_AREA_FNS = 8


def area_walk(paths):
    """Mirror placement.rs::AreaWalk — adaptive base area for any path."""
    from collections import Counter as _C
    counts = _C()
    for p in paths:
        dirs = p.split("/")[:-1]
        for i in range(len(dirs) + 1):
            counts["/".join(dirs[:i])] += 1

    def area(path):
        dirs = path.split("/")[:-1]
        prefix = ""
        for d in dirs:
            nxt = f"{prefix}/{d}" if prefix else d
            if counts.get(nxt, 0) > MAX_CONTAINER_FRAC * counts.get(prefix, 0):
                prefix = nxt
                continue
            if counts.get(nxt, 0) >= MIN_AREA_FNS:
                return nxt
            return prefix
        return prefix
    return area
# reinvention two-tier fire rule (mirror redundant.rs)
NORMAL_SIM = 0.78
NORMAL_SUB = 0.40
NORMAL_CALLEE = 0.12
NORMAL_MIN_CALLEES = 2
STRONG_SIM = 0.70
STRONG_SUB = 0.52
STRONG_CALLEE = 0.30
STRONG_MIN_CALLEES = 3


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
    """Yield (lang, vecs, paths, placement_cfg, callees, subtoks, symbols) per language."""
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
        yield lang, vecs, ld["paths"], ld.get("placement", {}), callees, subtoks, ld["symbols"]


def cos(a, b):
    return sum(x * y for x, y in zip(a, b))


def analyse_lang(vecs, paths, placement, callees, subtoks, syms):
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
        single = (len(callees[candi]) == 1 and len(callees[matchi]) == 1
                  and callees[candi] == callees[matchi])
        normal = c >= NORMAL_SIM and ((both(NORMAL_MIN_CALLEES) and cj >= NORMAL_CALLEE)
                                      or sj >= NORMAL_SUB or single)
        strong = c >= STRONG_SIM and ((both(STRONG_MIN_CALLEES) and cj >= STRONG_CALLEE) or sj >= STRONG_SUB)
        rare = c >= STRONG_SIM and any(cdf.get(t, 0) <= rare_df for t in (callees[candi] & callees[matchi]))
        return normal or strong or rare

    # numpy neighbour search (identical semantics to the sorted-generator scan,
    # just vectorised so large corpora finish in seconds). Vectors are already
    # L2-normalised, so V @ V[qi] is the cosine row.
    V = np.asarray(vecs, dtype=np.float32)
    path_arr = np.asarray(paths)

    reinv_fire = reinv_eval = 0
    for qi in sample:
        if not is_reinvention_candidate(syms[qi], paths[qi]):
            continue
        s = V @ V[qi]
        s[path_arr == paths[qi]] = -1e30  # nearest *cross-file*
        bi = int(np.argmax(s))
        if s[bi] <= -1e29:
            continue
        reinv_eval += 1
        if syms[qi].lower() == syms[bi].lower():
            continue
        if fires(qi, bi):
            reinv_fire += 1

    place_recall_fire = place_of_fire = place_eval = 0
    n_areas = 0
    if placement.get("enabled"):
        k = placement["k"]
        z = placement["z"]
        amap = placement.get("area_map", {})
        walk = area_walk(paths)
        fn_area = [amap.get(walk(p), walk(p)) for p in paths]
        uniq = sorted(set(fn_area))
        n_areas = len(uniq)
    if n_areas >= 2:
        for qi in sample:
            s = V @ V[qi]
            s[qi] = -1e30
            kk = min(k, n - 1)
            top = np.argpartition(s, -kk)[-kk:]
            top = top[np.argsort(s[top])[::-1]]  # descending cosine
            nb = [fn_area[int(j)] for j in top]
            if len(nb) < MIN_NEIGH:
                continue
            place_eval += 1
            cnt = Counter(nb)
            mx = max(cnt.values())
            modal = min(a for a, c in cnt.items() if c == mx)

            def place_fires(claimed, claimed_dir):
                # prod gate: placement can only judge a location with precedent.
                if claimed_dir not in index_dirs:
                    return False
                return modal != claimed and sum(1 for a in nb if a == claimed) <= z

            actual = fn_area[qi]
            if place_fires(actual, parent_dir(paths[qi])):  # in-place → over-fire
                place_of_fire += 1
            # transplant into a random foreign area, reusing a real dir of that area
            # so the prod new-dir gate is satisfied (relocation, not a new dir).
            foreign = random.choice([a for a in uniq if a != actual])
            foreign_dir = next((parent_dir(p) for p, fa in zip(paths, fn_area)
                                if fa == foreign and "/" in p), foreign)
            if place_fires(foreign, foreign_dir):           # transplanted → recall
                place_recall_fire += 1

    return {"functions": n, "reinv_fire": reinv_fire, "reinv_eval": reinv_eval,
            "place_recall_fire": place_recall_fire, "place_of_fire": place_of_fire,
            "place_eval": place_eval, "n_areas": n_areas}


def main():
    if not os.path.exists(IDX):
        print(f"no index at {IDX}", file=sys.stderr); sys.exit(1)
    agg = Counter()
    langs = []
    for lang, vecs, paths, placement, callees, subtoks, syms in load_langs(IDX):
        r = analyse_lang(vecs, paths, placement, callees, subtoks, syms)
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
