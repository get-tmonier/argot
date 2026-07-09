#!/usr/bin/env python3
"""Architecture-graph foreignness probe (cheap, no scorer change).

Hypothesis: an LLM pastes code that creates an INTERNAL module-dependency edge the
repo's own topology never has — a layer it never crosses, or a dependency DIRECTION
it never uses (models/ importing views/). Discrete like an import, and invisible to
the base vocabulary gate (the imported module is the repo's OWN code).

Questions, cheapest first:
  Q1 SIGNAL EXISTS? Are module graphs strongly DIRECTIONAL (layered) — many pairs
     where A->B is frequent and B->A never? A dense tangle has no layering to violate.
  Q2 GATABLE FP? Split files 70/30; fit the layer-edge set on 70%; of held-out files'
     cross-layer edges, what fraction are:
       novel     = 0-usage in fit               (any new edge; organic growth — noisy)
       reversal  = novel AND reverse attested    (violates an established direction)
       sink-out  = novel AND from a fit SINK layer (a leaf that only gets imported,
                   now importing out — a boundary break)
     The clean gatable tell = reversal ∪ sink-out. Low on real held-out ⇒ gatable.

Domain-blind: "layer" = the path component under the package root (never a hardcoded
layer name). Internal edges only (external deps are the base gate's job).

Usage: source .venv/bin/activate && python benchmarks/arch_graph_probe.py
"""
import ast, glob, os, sys, random
from collections import Counter, defaultdict

random.seed(0)
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "benchmarks", "data")
CORPORA = ["scrapy", "rich", "faker", "fastapi", "wagtail", "saleor", "dagster"]
SKIP_DIRS = ("/test", "/tests/", "/examples/", "/example/", "/docs/", "/doc/",
             "/.buildkite/", "/.github/", "/integration_tests/", "/helm/",
             "/migrations/", "/vendor/", "/third_party/")


def is_skip(path):
    return any(s in path for s in SKIP_DIRS)


def package_roots(repo):
    """Every package root anywhere in the tree: a dir with __init__.py whose parent
    has none. Returns {abs_dir: pkg_name}. Skips test/example/infra trees."""
    roots = {}
    for init in glob.glob(f"{repo}/**/__init__.py", recursive=True):
        d = os.path.dirname(init)
        if is_skip(d + "/"):
            continue
        parent = os.path.dirname(d)
        if not os.path.exists(os.path.join(parent, "__init__.py")):
            roots[d] = os.path.basename(d)
    return roots


def enclosing_root(path, roots):
    """Nearest ancestor package root dir of `path` (a file), or None."""
    d = os.path.dirname(path)
    while True:
        if d in roots:
            return d
        parent = os.path.dirname(d)
        if parent == d:
            return None
        d = parent


def imports_of(tree):
    out = []
    for n in ast.walk(tree):
        if isinstance(n, ast.Import):
            out += [(a.name, 0) for a in n.names]
        elif isinstance(n, ast.ImportFrom):
            out.append((n.module or "", n.level))
    return out


def corpus_edges(corp):
    repo = os.path.join(DATA, corp, ".repo")
    roots = package_roots(repo)
    if not roots:
        return None
    pkg_names = set(roots.values())
    per_file = []
    for root_dir in roots:
        for f in glob.glob(f"{root_dir}/**/*.py", recursive=True):
            if is_skip(f):
                continue
            er = enclosing_root(f, roots)
            if er is None:
                continue
            rel = os.path.relpath(f, er)
            parts = rel.split(os.sep)
            src_layer = parts[0] if len(parts) > 1 else "__root__"
            src_pkg_parts = rel.split(os.sep)
            try:
                tree = ast.parse(open(f, encoding="utf-8", errors="ignore").read())
            except Exception:
                continue
            edges = set()
            for mod, level in imports_of(tree):
                if level == 0:
                    p = mod.split(".") if mod else []
                    if not p or p[0] not in pkg_names:
                        continue  # external dep — base gate's job
                    tgt = p[1] if len(p) > 1 else "__root__"
                else:
                    base = src_pkg_parts[:-1]
                    up = level - 1
                    base = base[: len(base) - up] if up <= len(base) else []
                    tail = mod.split(".") if mod else []
                    full = base + tail
                    tgt = full[0] if full else "__root__"
                if tgt != src_layer:
                    edges.add((src_layer, tgt))
            if edges:
                per_file.append(edges)
    return per_file


def main():
    print(f"{'corpus':10} {'lyr':>4} {'xedge':>6} {'asym%':>6} "
          f"{'novel%':>7} {'rev%':>6} {'sink%':>6} {'FP%':>6} {'catch%':>7}")
    print("-" * 68)
    agg = []
    for corp in CORPORA:
        pf = corpus_edges(corp)
        if not pf:
            print(f"{corp:10}  (no package roots)")
            continue
        w = Counter()
        for edges in pf:
            for e in edges:
                w[e] += 1
        layers = {l for (a, b) in w for l in (a, b)}
        pair_mass = defaultdict(lambda: [0, 0])
        for (a, b), c in w.items():
            key = tuple(sorted((a, b)))
            pair_mass[key][0 if (a, b) == key else 1] += c
        dom = sum(max(v) for v in pair_mass.values())
        back = sum(min(v) for v in pair_mass.values())
        asym = 100 * dom / (dom + back) if (dom + back) else 0

        files = pf[:]; random.shuffle(files)
        ntr = int(0.7 * len(files))
        fit = set()
        for edges in files[:ntr]:
            fit |= edges
        # sink layers in the fit graph: cross-layer in-edges > 0, out-edges == 0
        out_deg, in_deg = Counter(), Counter()
        for (a, b) in fit:
            out_deg[a] += 1
            in_deg[b] += 1
        sinks = {l for l in (set(out_deg) | set(in_deg))
                 if in_deg[l] > 0 and out_deg[l] == 0}
        novel = rev = sink = clean = tot = 0
        for edges in files[ntr:]:
            for (a, b) in edges:
                tot += 1
                if (a, b) not in fit:
                    novel += 1
                    is_rev = (b, a) in fit
                    is_sink = a in sinks
                    if is_rev:
                        rev += 1
                    if is_sink:
                        sink += 1
                    if is_rev or is_sink:
                        clean += 1
        f = lambda x: 100 * x / tot if tot else 0
        # CATCH estimate (coverage): over the FULL graph's cross-layer NON-edges,
        # what share would the reversal∪sink rule flag if an LLM created it? Uses the
        # full-graph sinks/edges (not the 70% fit) — the deployed rule's coverage.
        fout, fin = Counter(), Counter()
        for (a, b) in w:
            fout[a] += 1
            fin[b] += 1
        fsinks = {l for l in layers if fin[l] > 0 and fout[l] == 0}
        nonedge = cover = 0
        for a in layers:
            for b in layers:
                if a != b and (a, b) not in w:
                    nonedge += 1
                    if (b, a) in w or a in fsinks:
                        cover += 1
        catch = 100 * cover / nonedge if nonedge else 0
        agg.append((f(novel), f(clean), catch))
        print(f"{corp:10} {len(layers):4d} {len(w):6d} {asym:5.0f}% "
              f"{f(novel):6.1f}% {f(rev):5.1f}% {f(sink):5.1f}% "
              f"{f(clean):6.1f}% {catch:6.0f}%")
    if agg:
        import statistics as st
        print("-" * 68)
        print(f"{'MEAN':10} {'':16} {'novel':>6} "
              f"{st.mean(a[0] for a in agg):6.1f}%{'':13} "
              f"FP {st.mean(a[1] for a in agg):.1f}%  catch {st.mean(a[2] for a in agg):.0f}%")
    print("\nFP% = reversal ∪ sink-out over-fire on the repo's own held-out files "
          "(want ≤5%). catch% = coverage: of cross-layer edges the repo does NOT "
          "have, the share the rule flags if an LLM adds it (uniform-violation model).")


if __name__ == "__main__":
    main()
