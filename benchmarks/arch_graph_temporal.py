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


def main():
    print(f"{'corpus':10} {'commits':>8} {'fires':>6} {'over-fire%':>11} "
          f"{'(rev/sink)':>12}")
    print("-" * 52)
    for corp in CORPORA:
        repo = os.path.join(DATA, corp, ".repo")
        head = git(repo, "rev-parse", "HEAD").strip()
        fit = git(repo, "rev-parse", f"{head}~{WINDOW}").strip()
        if not fit:
            print(f"{corp:10}  (history < window)")
            continue
        roots = package_roots_at(repo, fit)
        pkg_names = set(roots.values())
        # build fit graph from the fit tree
        fit_files = [f for f in git(repo, "ls-tree", "-r", "--name-only", fit).splitlines()
                     if f.endswith(".py") and "test" not in f.lower()
                     and "/migrations/" not in f]
        G = set()
        for f in fit_files:
            c = show(repo, fit, f)
            if c:
                G |= file_edges(f, c, roots, pkg_names)
        # sinks + reversibles from the fit graph
        out_deg, in_deg = Counter(), Counter()
        for (a, b) in G:
            out_deg[a] += 1
            in_deg[b] += 1
        sinks = {l for l in (set(out_deg) | set(in_deg)) if in_deg[l] > 0 and out_deg[l] == 0}

        replay = git(repo, "rev-list", "--no-merges", "--reverse",
                     f"{fit}..{head}").split()
        fires = 0
        rev_n = sink_n = 0
        for sha in replay:
            changed = git(repo, "diff-tree", "--no-commit-id", "--name-only", "-r",
                          sha).splitlines()
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
                    is_rev = (b, a) in G
                    is_sink = a in sinks
                    if is_rev or is_sink:
                        fired = True
                        if is_rev:
                            rev_n += 1
                        elif is_sink:
                            sink_n += 1
            if fired:
                fires += 1
        n = len(replay)
        pct = 100 * fires / n if n else 0
        print(f"{corp:10} {n:8d} {fires:6d} {pct:10.1f}% {f'{rev_n}/{sink_n}':>12}")
    print("\nover-fire% = share of real clean commits that introduce a "
          "direction-reversal or sink-out edge (the gatable tell). Want ≤5%.")


if __name__ == "__main__":
    main()
