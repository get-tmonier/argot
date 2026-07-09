#!/usr/bin/env python3
"""Architecture-graph — raise catch to >=85% via finer GRANULARITY + near-sink tell.

Top-layer granularity caps coverage catch at ~77% (importing a foundational module from a
new top-layer is usually legit growth, not a violation). A finer graph (package/subpackage,
G path components) surfaces intra-layer directional constraints that top-layer hides —
more real violations become catchable. Also test a NEAR-sink tell (a mostly-imported layer,
out/in mass ratio < r, importing out) generalizing the strict sink.

Sweeps granularity G in {1,2,3} x rule in {reversal∪sink, +near-sink}. FP = 70/30 file
split; catch = popularity-weighted coverage. Want meanCatch>=85, worstFP<=5.
Usage: source .venv/bin/activate && python benchmarks/arch_graph_gran.py
"""
import ast, glob, os, random, sys
import numpy as np
from collections import Counter

random.seed(0)
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "benchmarks", "data")
CORPORA = ["scrapy", "rich", "faker", "fastapi", "wagtail", "saleor", "dagster"]
SKIP = ("test", "example", "doc", "migration", "vendor", "third_party", ".buildkite")


def pkg_roots(repo):
    roots = {}
    for init in glob.glob(f"{repo}/**/__init__.py", recursive=True):
        d = os.path.dirname(init)
        if any(s in d.lower() for s in SKIP):
            continue
        if not os.path.exists(os.path.join(os.path.dirname(d), "__init__.py")):
            roots[d] = os.path.basename(d)
    return roots


def enclosing(path, roots):
    d = os.path.dirname(path)
    while True:
        if d in roots:
            return d
        p = os.path.dirname(d)
        if p == d:
            return None
        d = p


def dir_layer(file_parts, g):
    """Layer of a FILE at granularity g: first g components of its dir (drop filename)."""
    d = file_parts[:-1]  # drop the filename component
    return "/".join(d[:g]) if d else "__root__"


def mod_layer(mod_parts, g):
    """Layer of a MODULE target at granularity g: first g components (no filename)."""
    return "/".join(mod_parts[:g]) if mod_parts else "__root__"


def corpus_edges(corp, g):
    repo = os.path.join(DATA, corp, ".repo")
    roots = pkg_roots(repo)
    if not roots:
        return None
    names = set(roots.values())
    per_file = []
    for rd in roots:
        for f in glob.glob(f"{rd}/**/*.py", recursive=True):
            if any(s in f.lower() for s in SKIP):
                continue
            er = enclosing(f, roots)
            if er is None:
                continue
            rel = os.path.relpath(f, er).split(os.sep)
            src = dir_layer(rel, g)
            try:
                tree = ast.parse(open(f, encoding="utf-8", errors="ignore").read())
            except Exception:
                continue
            edges = set()
            for n in ast.walk(tree):
                specs = []
                if isinstance(n, ast.Import):
                    specs = [(a.name, 0) for a in n.names]
                elif isinstance(n, ast.ImportFrom):
                    specs = [(n.module or "", n.level)]
                for mod, level in specs:
                    if level == 0:
                        p = [x for x in mod.split(".") if x]
                        if not p or p[0] not in names:
                            continue
                        tgt = mod_layer(p[1:], g)  # under the package name
                    else:
                        base = rel[:-1]
                        up = level - 1
                        base = base[: len(base) - up] if up <= len(base) else []
                        base = base + [x for x in mod.split(".") if x]
                        tgt = mod_layer(base, g)
                    if tgt != src:
                        edges.add((src, tgt))
            if edges:
                per_file.append(edges)
    return per_file


def analyze(per_file, r=0.25, delta=None):
    """Rule: fire a novel edge a->b if reversal (b->a) OR a is a near-sink (out-mass
    fraction <= r) OR rank(a)-rank(b) >= delta (against the foundational gradient)."""
    W = Counter()
    for edges in per_file:
        for e in edges:
            W[e] += 1

    def ctx(G):
        om, im = Counter(), Counter()
        for (a, b), c in G.items():
            om[a] += c
            im[b] += c
        layers = {l for e in G for l in e}
        near = {l for l in layers if im[l] > 0 and om[l] / (im[l] + om[l]) <= r}
        rk = {l: (im[l] / (im[l] + om[l]) if (im[l] + om[l]) else 0.0) for l in layers}
        return near, rk, im

    def fires(a, b, G, near, rk):
        if (b, a) in G or a in near:
            return True
        if delta is not None and rk.get(a, 0.0) - rk.get(b, 0.0) >= delta:
            return True
        return False

    # catch: popularity-weighted coverage over plausible missing edges
    near, rk, im = ctx(W)
    layers = list({l for e in W for l in e})
    num = den = 0.0
    for a in layers:
        for b in layers:
            if a != b and (a, b) not in W and im[b] > 0:
                den += im[b]
                if fires(a, b, W, near, rk):
                    num += im[b]
    catch = 100 * num / den if den else 0
    return catch


def main():
    G = 1  # top-level layer (real temporal FP already ≤2.6% at G=1)
    # cache edges per corpus once
    pfs = {c: corpus_edges(c, G) for c in CORPORA}
    pfs = {c: pf for c, pf in pfs.items() if pf}
    print(f"{'r':>5} {'delta':>6} {'meanCatch%':>11} {'minCatch%':>10}  (G={G}, "
          f"popularity-weighted coverage)")
    print("-" * 50)
    for r in (0.25, 0.4, 0.5):
        for delta in (None, 0.5, 0.35, 0.25):
            cs = [analyze(pf, r=r, delta=delta) for pf in pfs.values()]
            dtag = "none" if delta is None else f"{delta:.2f}"
            print(f"{r:5.2f} {dtag:>6} {np.mean(cs):10.0f}% {min(cs):9.0f}%")
    print("\nEach corpus's catch listed for the current-winner config below:")
    r, delta = 0.4, 0.35
    for c, pf in pfs.items():
        print(f"  {c:10} {analyze(pf, r=r, delta=delta):.0f}%")
    print(f"(shown config r={r}, delta={delta}. Pick the knee, then real-temporal-FP check.)")


if __name__ == "__main__":
    main()
