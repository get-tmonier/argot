#!/usr/bin/env python3
"""Architecture-graph foreignness — cross-language spot-check (Go + TypeScript).

Confirms the Python cheap-probe result generalizes: are non-Python module graphs also
strongly DIRECTIONAL (layered), and is the reversal∪sink tell also low-FP? Go is the
cleanest possible test (packages == directories, explicit import paths); TS tests
relative-import resolution.

Heuristic import extraction (regex, no tree-sitter — this is a spot-check, not the
production extractor). Same graph analysis as arch_graph_probe.py: layer = first path
component under the source root; internal edges only; FP = 70/30 file-split over-fire of
the clean tell; catch = popularity-weighted realistic-violation coverage.

Usage: source .venv/bin/activate && python benchmarks/arch_graph_xlang.py
"""
import os, re, glob, sys, random
from collections import Counter, defaultdict

random.seed(0)
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "benchmarks", "data")

GO = ["hugo", "gh-cli"]
TS = ["hono", "outline", "excalidraw"]
SKIP = ("/test", "_test.", "/vendor/", "/node_modules/", "/dist/", "/examples/",
        "/example/", ".test.", ".spec.", "/docs/", "/testdata/")


def is_skip(p):
    return any(s in p for s in SKIP)


def go_edges(corp):
    repo = os.path.join(DATA, corp, ".repo")
    # module path from go.mod
    mod = None
    try:
        for line in open(os.path.join(repo, "go.mod"), encoding="utf-8"):
            if line.startswith("module "):
                mod = line.split()[1].strip()
                break
    except OSError:
        return None
    if not mod:
        return None
    per_file = []
    imp_re = re.compile(r'"([^"]+)"')
    for f in glob.glob(f"{repo}/**/*.go", recursive=True):
        if is_skip(f):
            continue
        rel = os.path.relpath(os.path.dirname(f), repo)
        src_layer = rel.split(os.sep)[0] if rel != "." else "__root__"
        try:
            txt = open(f, encoding="utf-8", errors="ignore").read()
        except OSError:
            continue
        # import block(s)
        edges = set()
        for m in re.finditer(r'import\s*\((.*?)\)', txt, re.S):
            for q in imp_re.findall(m.group(1)):
                if q.startswith(mod):
                    sub = q[len(mod):].lstrip("/")
                    tgt = sub.split("/")[0] if sub else "__root__"
                    if tgt and tgt != src_layer:
                        edges.add((src_layer, tgt))
        for m in re.finditer(r'import\s+"([^"]+)"', txt):
            q = m.group(1)
            if q.startswith(mod):
                sub = q[len(mod):].lstrip("/")
                tgt = sub.split("/")[0] if sub else "__root__"
                if tgt and tgt != src_layer:
                    edges.add((src_layer, tgt))
        if edges:
            per_file.append(edges)
    return per_file


def ts_source_root(rel):
    """Strip a leading src/ or packages/<name>/(src/)? so 'layer' is meaningful."""
    parts = rel.split(os.sep)
    if parts and parts[0] == "packages" and len(parts) > 2:
        parts = parts[2:]
        if parts and parts[0] == "src":
            parts = parts[1:]
    elif parts and parts[0] in ("src", "app"):
        # keep 'app' as a layer for outline-style repos; strip only 'src'
        if parts[0] == "src":
            parts = parts[1:]
    return parts


def ts_edges(corp):
    repo = os.path.join(DATA, corp, ".repo")
    per_file = []
    imp_re = re.compile(r'''(?:import|export)\b[^;'"]*?from\s*['"]([^'"]+)['"]''')
    req_re = re.compile(r'''require\(\s*['"]([^'"]+)['"]\s*\)''')
    files = [f for f in glob.glob(f"{repo}/**/*.ts", recursive=True)
             + glob.glob(f"{repo}/**/*.tsx", recursive=True) if not is_skip(f)]
    for f in files:
        rel = os.path.relpath(f, repo)
        src_parts = ts_source_root(os.path.dirname(rel) + os.sep + "x")[:-1]
        src_layer = src_parts[0] if src_parts else "__root__"
        try:
            txt = open(f, encoding="utf-8", errors="ignore").read()
        except OSError:
            continue
        edges = set()
        for spec in imp_re.findall(txt) + req_re.findall(txt):
            if not spec.startswith("."):
                continue  # external / package import
            # resolve relative to the file's dir, then strip source root
            tgt_abs = os.path.normpath(os.path.join(os.path.dirname(rel), spec))
            tparts = ts_source_root(tgt_abs)
            tgt_layer = tparts[0] if tparts else "__root__"
            if tgt_layer and tgt_layer != src_layer and not tgt_layer.startswith(".."):
                edges.add((src_layer, tgt_layer))
        if edges:
            per_file.append(edges)
    return per_file


def analyze(name, per_file):
    if not per_file:
        print(f"{name:12}  (no edges)")
        return
    w = Counter()
    for edges in per_file:
        for e in edges:
            w[e] += 1
    layers = {l for (a, b) in w for l in (a, b)}
    pair = defaultdict(lambda: [0, 0])
    for (a, b), c in w.items():
        key = tuple(sorted((a, b)))
        pair[key][0 if (a, b) == key else 1] += c
    dom = sum(max(v) for v in pair.values()); back = sum(min(v) for v in pair.values())
    asym = 100 * dom / (dom + back) if dom + back else 0

    files = per_file[:]; random.shuffle(files)
    ntr = int(0.7 * len(files))
    fit = set()
    for edges in files[:ntr]:
        fit |= edges
    od, idg = Counter(), Counter()
    for (a, b) in fit:
        od[a] += 1; idg[b] += 1
    sinks = {l for l in (set(od) | set(idg)) if idg[l] > 0 and od[l] == 0}
    novel = clean = tot = 0
    for edges in files[ntr:]:
        for (a, b) in edges:
            tot += 1
            if (a, b) not in fit:
                novel += 1
                if (b, a) in fit or a in sinks:
                    clean += 1
    fp = 100 * clean / tot if tot else 0
    novel_pct = 100 * novel / tot if tot else 0
    # realistic catch
    fout, fin = Counter(), Counter()
    for (a, b), c in w.items():
        fout[a] += 1; fin[b] += c
    fsinks = {l for l in layers if fin[l] > 0 and fout[l] == 0}
    num = den = 0.0
    for a in layers:
        for b in layers:
            if a != b and (a, b) not in w and fin[b] > 0:
                den += fin[b]
                if (b, a) in w or a in fsinks:
                    num += fin[b]
    catch = 100 * num / den if den else 0
    print(f"{name:12} {len(layers):4d} {len(w):6d} {asym:5.0f}% "
          f"{novel_pct:6.1f}% {fp:6.1f}% {catch:6.0f}%")


def main():
    print(f"{'corpus':12} {'lyr':>4} {'edge':>6} {'asym%':>6} "
          f"{'novel%':>7} {'FP%':>6} {'catch%':>7}")
    print("-" * 52)
    print("# Go (packages == directories, explicit imports)")
    for c in GO:
        analyze(c, go_edges(c))
    print("# TypeScript (relative-import resolution)")
    for c in TS:
        analyze(c, ts_edges(c))
    print("\nSame tell as Python: FP = reversal∪sink over-fire (70/30 file split), "
          "catch = popularity-weighted realistic-violation coverage. Want FP≤5%.")


if __name__ == "__main__":
    main()
