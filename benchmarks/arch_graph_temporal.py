#!/usr/bin/env python3
"""Architecture-graph foreignness — REAL temporal-holdout FP (the decisive number).

The file-split FP (arch_graph_probe.py) is a proxy. This replays ACTUAL clean commits:
fit the layer-dependency graph at HEAD~window, then for every non-merge commit after it,
attribute the edges that commit ADDS (file edges at sha minus at sha^) and count those
that are the clean tell (direction-reversal ∪ sink-out) vs the fit graph. Commit-level
over-fire = share of clean commits that introduce such an edge — the honest false-alarm
rate a maintainer would feel. Python corpora (best extractor).

Usage: source .venv/bin/activate && python benchmarks/arch_graph_temporal.py
"""
import ast, os, subprocess, sys
from collections import Counter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "benchmarks", "data")
CORPORA = ["scrapy", "rich", "faker", "fastapi", "wagtail", "saleor"]
WINDOW = 150


def git(repo, *args):
    return subprocess.run(["git", "-C", repo, *args], capture_output=True,
                          text=True).stdout


def package_roots_at(repo, sha):
    """Package root dirs (dir with __init__.py, parent without) in the fit tree."""
    files = git(repo, "ls-tree", "-r", "--name-only", sha).splitlines()
    inits = {os.path.dirname(f) for f in files if f.endswith("__init__.py")}
    skip = ("test", "example", "doc", "migrations", "vendor", "third_party", ".buildkite")
    roots = {}
    for d in inits:
        if any(s in d.lower() for s in skip):
            continue
        parent = os.path.dirname(d)
        if parent not in inits:
            roots[d] = os.path.basename(d)
    return roots


def enclosing_root(path, roots):
    d = os.path.dirname(path)
    while True:
        if d in roots:
            return d
        p = os.path.dirname(d)
        if p == d:
            return None
        d = p


def file_edges(path, content, roots, pkg_names):
    """Cross-layer edges (src_layer, tgt_layer) introduced by one file's imports."""
    er = enclosing_root(path, roots)
    if er is None:
        return set()
    rel = os.path.relpath(path, er)
    parts = rel.split(os.sep)
    src_layer = parts[0] if len(parts) > 1 else "__root__"
    try:
        tree = ast.parse(content)
    except Exception:
        return set()
    edges = set()
    for n in ast.walk(tree):
        mods = []
        if isinstance(n, ast.Import):
            mods = [(a.name, 0) for a in n.names]
        elif isinstance(n, ast.ImportFrom):
            mods = [(n.module or "", n.level)]
        for mod, level in mods:
            if level == 0:
                p = mod.split(".") if mod else []
                if not p or p[0] not in pkg_names:
                    continue
                tgt = p[1] if len(p) > 1 else "__root__"
            else:
                base = parts[:-1]
                up = level - 1
                base = base[: len(base) - up] if up <= len(base) else []
                tail = mod.split(".") if mod else []
                full = base + tail
                tgt = full[0] if full else "__root__"
            if tgt != src_layer:
                edges.add((src_layer, tgt))
    return edges


def show(repo, sha, path):
    r = subprocess.run(["git", "-C", repo, "show", f"{sha}:{path}"],
                       capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None


def build(repo, corp, near, ratio=0.25):
    """Return (fit graph, sinks, replay, pkg_names, roots, head) for a corpus."""
    head = git(repo, "rev-parse", "HEAD").strip()
    fit = git(repo, "rev-parse", f"{head}~{WINDOW}").strip()
    if not fit:
        return None
    roots = package_roots_at(repo, fit)
    pkg_names = set(roots.values())
    fit_files = [f for f in git(repo, "ls-tree", "-r", "--name-only", fit).splitlines()
                 if f.endswith(".py") and "test" not in f.lower()
                 and "/migrations/" not in f]
    G = Counter()
    for f in fit_files:
        c = show(repo, fit, f)
        if c:
            for e in file_edges(f, c, roots, pkg_names):
                G[e] += 1
    out_m, in_m = Counter(), Counter()
    for (a, b), c in G.items():
        out_m[a] += c
        in_m[b] += c
    layers = {l for e in G for l in e}
    if near:
        sinks = {l for l in layers if in_m[l] > 0 and out_m[l] / (in_m[l] + out_m[l]) <= ratio}
    else:
        sinks = {l for l in layers if in_m[l] > 0 and out_m[l] == 0}
    replay = git(repo, "rev-list", "--no-merges", "--reverse", f"{fit}..{head}").split()
    return set(G), sinks, replay, pkg_names, roots, head


def real_fp(repo, corp, near, ratio=0.25):
    b = build(repo, corp, near, ratio)
    if b is None:
        return None
    G, sinks, replay, pkg_names, roots, head = b
    fires = 0
    for sha in replay:
        changed = git(repo, "diff-tree", "--no-commit-id", "--name-only", "-r", sha).splitlines()
        fired = False
        for path in changed:
            if not path.endswith(".py") or "test" in path.lower() or "/migrations/" in path:
                continue
            cur = show(repo, sha, path)
            if cur is None:
                continue
            par = show(repo, f"{sha}~1", path) or ""
            added = file_edges(path, cur, roots, pkg_names) - file_edges(path, par, roots, pkg_names)
            for (a, b) in added:
                if (a, b) in G:
                    continue
                if (b, a) in G or a in sinks:
                    fired = True
        if fired:
            fires += 1
    n = len(replay)
    return (n, fires, 100 * fires / n if n else 0)


def main():
    print(f"{'rule':22} {'commits':>8} {'fires':>6} {'agg-FP%':>8} {'worst%':>7}")
    print("-" * 54)
    for near, ratio in [(False, 0.0), (True, 0.25), (True, 0.4), (True, 0.5)]:
        tag = "strict-sink" if not near else f"near-sink r={ratio}"
        tot_c = tot_f = 0
        worst = 0.0
        for corp in CORPORA:
            repo = os.path.join(DATA, corp, ".repo")
            r = real_fp(repo, corp, near, ratio)
            if r is None:
                continue
            n, f, pct = r
            tot_c += n
            tot_f += f
            worst = max(worst, pct)
        agg = 100 * tot_f / tot_c if tot_c else 0
        print(f"{tag:22} {tot_c:8d} {tot_f:6d} {agg:7.2f}% {worst:6.1f}%")
    print("\nREAL temporal over-fire (per-commit) — pick the most aggressive near-sink "
          "ratio whose worst-corpus FP stays ≤5%; that maximizes catch.")


if __name__ == "__main__":
    main()
