#!/usr/bin/env python3
"""Foreign-STRUCTURE gate — cross-family de-circularized catch + k lever for over-fire.

Two loose ends closed:
  1. Circularity: earlier catch-distinct used bg_df on the SAME family (bigrams) the gate
     fires on. Here distinctness is defined on PRODUCTIONS (independent family); the gate
     fires on BIGRAMS -> non-circular cross-family catch.
  2. Young-repo over-fire (faker/fastapi ~8-9% at k=1): require >=k native-absent globally-
     common bigrams to fire. Sweep k for an operating point with over-fire<=5% on EVERY
     corpus while production-distinct catch stays high.

  FIRE(hunk)   := #{ bigrams 0-usage in repo(train) with bg_df_bigram >= tau } >= k
  DISTINCT pos := home-idiomatic foreign window (all bigrams home-df>=3) whose loudest
                  native-absent PRODUCTION has bg_df_prod >= 0.5  (independent family)

Domain-blind node-kinds only. Usage: source .venv/bin/activate && python this.py
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
KS = [1, 2, 3]


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


def wfeat(win, fn):
    ps = []
    for s in win:
        ps += fn(s)
    return ps


def main():
    print("loading ...", file=sys.stderr)
    RAW = {c: load_fns(c) for c in CORPORA}
    for c in CORPORA:
        print(f"  {c}: {len(RAW[c])}", file=sys.stderr)

    # repo fn-doc-freq for both families + whole-repo production vocab (for distinctness)
    BDF, PDF, PVOCAB = {}, {}, {}
    for c in CORPORA:
        bd, pd_, pv = Counter(), Counter(), set()
        for body in RAW[c]:
            bset, pset = set(), set()
            for s in body:
                bset |= set(bigrams(s)); pset |= set(prods(s))
            for g in bset:
                bd[g] += 1
            for g in pset:
                pd_[g] += 1
            pv |= pset
        BDF[c], PDF[c], PVOCAB[c] = bd, pd_, pv

    for k in KS:
        print(f"\n### require >= {k} native-absent globally-common bigrams (tau={TAU})")
        print(f"{'native':10} {'over-fire':>10} {'catch-distinct(xfam)':>21} "
              f"{'#distinct':>10}")
        OF, CD = [], []
        for native in CORPORA:
            bodies = RAW[native][:]; random.shuffle(bodies)
            ntr = int(0.7 * len(bodies)); tr, te = bodies[:ntr], bodies[ntr:]
            bvocab = Counter()
            for body in tr:
                s0 = set()
                for s in body:
                    s0 |= set(bigrams(s))
                for g in s0:
                    bvocab[g] += 1
            bvocab = {g for g, n in bvocab.items() if n >= 1}
            others = [c for c in CORPORA if c != native]
            bgb = Counter()
            for c in others:
                for g in BDF[c]:
                    bgb[g] += 1
            bg_b = {g: v / len(others) for g, v in bgb.items()}
            bgp = Counter()
            for c in others:
                for g in PDF[c]:
                    bgp[g] += 1
            bg_p = {g: v / len(others) for g, v in bgp.items()}
            npvocab = PVOCAB[native]

            def nfire(ps):
                return sum(1 for g in set(ps)
                           if g not in bvocab and bg_b.get(g, 0.0) >= TAU)

            neg = np.array([nfire(wfeat(w, bigrams)) for w in windows(te, W)])
            OF.append(100 * (neg >= k).mean())

            fired, ndist = [], 0
            for c in others:
                homedf = BDF[c]
                for w in windows(RAW[c][:350], W):
                    bps = wfeat(w, bigrams)
                    if not bps:
                        continue
                    if not all(homedf.get(g, 0) >= 3 for g in set(bps)):
                        continue  # home-idiomatic
                    pps = wfeat(w, prods)
                    dist = any(bg_p.get(g, 0.0) >= 0.5 and g not in npvocab
                               for g in set(pps))
                    if not dist:
                        continue
                    ndist += 1
                    fired.append(nfire(bps) >= k)
            CD.append(100 * np.mean(fired) if fired else np.nan)
            print(f"{native:10} {OF[-1]:9.1f}% {CD[-1]:20.0f}% {ndist:10d}")
        print(f"{'MEAN':10} {np.mean(OF):9.1f}% {np.nanmean(CD):20.0f}%")
    print("\n(cross-family: distinctness on productions, gate on bigrams. Look for a k "
          "with over-fire<=5% on EVERY corpus while catch-distinct stays high.)")


if __name__ == "__main__":
    main()
