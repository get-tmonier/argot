#!/usr/bin/env python3
"""Foreign-STRUCTURE gate — UNION of families (last lever before declaring the floor).

Cross-family probe showed bigram and production gates catch DIFFERENT distinct idioms
(bigram catches only 28% of production-distinct ones). So a union gate should catch more.
This tests whether union breaks the recall ceiling at a gatable FP budget.

  FIRE(hunk) := (#native-absent globally-common BIGRAMS >= kb)
             OR (#native-absent globally-common PRODUCTIONS >= kp)          (tau=0.5)

Independent positive (no distinctness filter, no circularity): ALL home-idiomatic foreign
windows (every bigram home-df>=3) = real pasted foreign idioms, mundane included.
  over-fire = native held-out windows firing   (must be <=5% on EVERY corpus)
  catch-any = home-idiomatic foreign windows firing (the honest recall of a pasted idiom)

Grid over (kb, kp). If no cell reaches catch-any >>28% at over-fire<=5% everywhere, the
floor is irreducible. Domain-blind node-kinds only.
Usage: source .venv/bin/activate && python this.py
"""
import ast, glob, os, sys, random
import numpy as np
from collections import Counter

random.seed(0); np.random.seed(0)
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "benchmarks", "data")
CORPORA = ["scrapy", "rich", "faker", "fastapi", "wagtail", "saleor", "dagster"]
W = 3
TAU = 0.5
GRID = [(1, 1), (2, 1), (1, 2), (2, 2), (2, 3), (3, 2), (3, 3)]


def bigrams(node):
    out = []

    def walk(n, parent):
        k = type(n).__name__
        if parent is not None:
            out.append((parent, k))
        for c in ast.iter_child_nodes(n):
            walk(c, k)
    walk(node, None)
    return out


def prods(node):
    out = []

    def walk(n):
        kids = list(ast.iter_child_nodes(n))
        if kids:
            out.append((type(n).__name__, tuple(type(c).__name__ for c in kids[:8])))
        for c in kids:
            walk(c)
    walk(node)
    return out


def load_fns(corp, cap=4000):
    repo = os.path.join(DATA, corp, ".repo")
    out = []
    files = [f for f in glob.glob(f"{repo}/**/*.py", recursive=True)
             if "/test" not in f and "/migrations/" not in f]
    random.shuffle(files)
    for f in files:
        try:
            tree = ast.parse(open(f, encoding="utf-8", errors="ignore").read())
        except Exception:
            continue
        for n in ast.walk(tree):
            if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef)):
                body = [s for s in n.body if isinstance(s, ast.stmt)]
                if len(body) >= 3:
                    out.append(body)
        if len(out) >= cap:
            break
    return out


def windows(bodies, w):
    for body in bodies:
        for i in range(0, max(1, len(body) - w + 1)):
            yield body[i:i + w]


def wf(win, fn):
    ps = []
    for s in win:
        ps += fn(s)
    return ps


def main():
    print("loading ...", file=sys.stderr)
    RAW = {c: load_fns(c) for c in CORPORA}
    for c in CORPORA:
        print(f"  {c}: {len(RAW[c])}", file=sys.stderr)
    BDF, PDF = {}, {}
    for c in CORPORA:
        bd, pd_ = Counter(), Counter()
        for body in RAW[c]:
            bset, pset = set(), set()
            for s in body:
                bset |= set(bigrams(s)); pset |= set(prods(s))
            for g in bset:
                bd[g] += 1
            for g in pset:
                pd_[g] += 1
        BDF[c], PDF[c] = bd, pd_

    # precompute per-native structures
    per_native = {}
    for native in CORPORA:
        bodies = RAW[native][:]; random.shuffle(bodies)
        ntr = int(0.7 * len(bodies)); tr, te = bodies[:ntr], bodies[ntr:]
        bvoc, pvoc = Counter(), Counter()
        for body in tr:
            b0, p0 = set(), set()
            for s in body:
                b0 |= set(bigrams(s)); p0 |= set(prods(s))
            for g in b0:
                bvoc[g] += 1
            for g in p0:
                pvoc[g] += 1
        bvoc = {g for g, n in bvoc.items() if n >= 1}
        pvoc = {g for g, n in pvoc.items() if n >= 1}
        others = [c for c in CORPORA if c != native]
        bgb = Counter(); bgp = Counter()
        for c in others:
            for g in BDF[c]:
                bgb[g] += 1
            for g in PDF[c]:
                bgp[g] += 1
        bg_b = {g: v / len(others) for g, v in bgb.items()}
        bg_p = {g: v / len(others) for g, v in bgp.items()}

        # neg (native held-out) counts
        neg_nb, neg_np = [], []
        for w in windows(te, W):
            bps, pps = wf(w, bigrams), wf(w, prods)
            neg_nb.append(sum(1 for g in set(bps)
                              if g not in bvoc and bg_b.get(g, 0) >= TAU))
            neg_np.append(sum(1 for g in set(pps)
                              if g not in pvoc and bg_p.get(g, 0) >= TAU))
        # pos (home-idiomatic foreign) counts
        pos_nb, pos_np = [], []
        for c in others:
            homedf = BDF[c]
            for w in windows(RAW[c][:350], W):
                bps = wf(w, bigrams)
                if not bps or not all(homedf.get(g, 0) >= 3 for g in set(bps)):
                    continue
                pps = wf(w, prods)
                pos_nb.append(sum(1 for g in set(bps)
                                  if g not in bvoc and bg_b.get(g, 0) >= TAU))
                pos_np.append(sum(1 for g in set(pps)
                                  if g not in pvoc and bg_p.get(g, 0) >= TAU))
        per_native[native] = (np.array(neg_nb), np.array(neg_np),
                              np.array(pos_nb), np.array(pos_np))

    print(f"\n{'(kb,kp)':>9} {'maxOF':>7} {'meanOF':>7} {'catch-any':>10} "
          f"{'allOF<=5?':>9}")
    for kb, kp in GRID:
        OF, CA = [], []
        for native in CORPORA:
            nb, np_, pb, pp = per_native[native]
            of = 100 * ((nb >= kb) | (np_ >= kp)).mean()
            ca = 100 * ((pb >= kb) | (pp >= kp)).mean()
            OF.append(of); CA.append(ca)
        ok = "YES" if max(OF) <= 5 else "no"
        print(f"{str((kb, kp)):>9} {max(OF):6.1f}% {np.mean(OF):6.1f}% "
              f"{np.mean(CA):9.0f}% {ok:>9}")
    print("\n(catch-any = honest recall of a pasted real foreign idiom, mundane incl. "
          "Baseline bigram-only k=1 was ~13%. If union can't clear it much at "
          "over-fire<=5% everywhere, the gatable floor is irreducible.)")


if __name__ == "__main__":
    main()
