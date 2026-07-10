#!/usr/bin/env python3
"""Foreign-STRUCTURE novelty probe (cheap, no scorer change) — the fast iteration
harness for the foreign-structure detector goal. See
docs/research/evidence/foreign-structure-ast-pattern-signal.md.

Treats AST node-kind n-grams as a repo's "structural vocabulary" (domain-blind:
node kinds only, NO identifiers/strings/framework literals). A hunk's foreignness =
fraction of its n-grams that are 0-usage in the repo — the structural analog of
import/callee 0-usage detection. Measures how well each repo's own held-out
functions separate from OTHER repos' functions (the "generically-styled paste"
proxy): AUC + catch at a 5%/1% false-alarm budget.

Refine the feature/pattern extraction here and re-run until separation clears
~≥85%/≤5% before porting anything into argot-core. Python-only (uses `ast`); extend
to other languages via tree-sitter when the formulation is chosen.

Usage: benchmarks/.venv or project .venv → `python benchmarks/struct_ngram_probe.py`
"""
import ast, glob, os, sys, random
import numpy as np
from collections import Counter

random.seed(0); np.random.seed(0)
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "benchmarks", "data")
CORPORA = ["scrapy", "rich", "faker", "fastapi", "saleor", "wagtail"]


def ngrams_of_fn(node):
    """Domain-blind AST n-grams within a function: node-kind bigrams (parent->child)
    and trigrams (grandparent->parent->child). Pure structure — no names/literals."""
    bg, tg = [], []

    def walk(n, gp, p):
        k = type(n).__name__
        if p is not None:
            bg.append((p, k))
        if gp is not None and p is not None:
            tg.append((gp, p, k))
        for c in ast.iter_child_nodes(n):
            walk(c, p, k)
    walk(node, None, None)
    return bg, tg


def corpus_fns(corp, min_stmts=4, cap=4000):
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
                if sum(1 for _ in ast.walk(n) if isinstance(_, ast.stmt)) >= min_stmts:
                    out.append(ngrams_of_fn(n))
        if len(out) >= cap:
            break
    return out


def auc(pos, neg):
    allv = np.concatenate([pos, neg]); order = allv.argsort()
    ranks = np.empty_like(order, dtype=float); ranks[order] = np.arange(1, len(allv) + 1)
    rp = ranks[:len(pos)].sum()
    return (rp - len(pos) * (len(pos) + 1) / 2) / (len(pos) * len(neg))


def foreign_rate(fn, vocab, use_tg):
    grams = fn[1] if use_tg else fn[0]
    if not grams:
        return 0.0
    return sum(1 for g in grams if g not in vocab) / len(grams)


def main():
    print("extracting AST n-grams ...", file=sys.stderr)
    FN = {c: corpus_fns(c) for c in CORPORA}
    for c in CORPORA:
        print(f"  {c}: {len(FN[c])} fns", file=sys.stderr)

    for use_tg, label in [(False, "bigram"), (True, "trigram")]:
        print(f"\n=== structural {label} vocabulary: 0-usage-pattern rate, "
              f"native held-out vs foreign ===")
        print(f"{'native':10} {'foreign':10} {'AUC':>6} {'catch@5%FP':>11} {'catch@1%FP':>11}")
        agg = []
        for native in CORPORA:
            fns = FN[native][:]; random.shuffle(fns)
            ntr = int(0.7 * len(fns)); tr, te = fns[:ntr], fns[ntr:]
            cnt = Counter()
            for f in tr:
                for g in set(f[1] if use_tg else f[0]):
                    cnt[g] += 1
            vocab = {g for g, n in cnt.items() if n >= 2}  # attested = seen in >=2 fns
            s_native = np.array([foreign_rate(f, vocab, use_tg) for f in te])
            thr5, thr1 = np.percentile(s_native, 95), np.percentile(s_native, 99)
            for foreign in CORPORA:
                if foreign == native:
                    continue
                s_for = np.array([foreign_rate(f, vocab, use_tg) for f in FN[foreign]])
                a = auc(s_for, s_native)
                c5 = 100 * (s_for > thr5).mean(); c1 = 100 * (s_for > thr1).mean()
                agg.append((a, c5, c1))
                print(f"{native:10} {foreign:10} {a:6.2f} {c5:10.0f}% {c1:10.0f}%")
        A = np.array(agg)
        print(f"  MEAN {label}:  AUC {A[:,0].mean():.2f}   "
              f"catch@5%FP {A[:,1].mean():.0f}%   catch@1%FP {A[:,2].mean():.0f}%")
    print("\n(AUC 0.5 = no structural voice; >0.7 = real separable signal. "
          "Baseline aggregate-feature novelty scored 0.60; see evidence memo.)")


if __name__ == "__main__":
    main()
