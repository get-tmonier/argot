#!/usr/bin/env python3
"""Architecture-graph — push catch to >=85% at <=5% FP via a rank-gradient rule.

The reversal∪sink rule catches ~77% at ~1% FP — lots of FP headroom. The missed
violations are novel-forward edges. Generalize: rank each layer by how FOUNDATIONAL it
is, rank(l) = in_mass / (in_mass + out_mass) (a pure sink=1, pure source=0). Healthy
flow is app->foundational; a violation goes AGAINST the gradient. Fire a novel edge a->b
if reversal (b->a attested) OR rank(a) - rank(b) >= delta (a much more foundational than
b -> it should not import b). Sweep delta for the knee that clears 85%/5%.

Reuses arch_graph_probe's Python edge extraction. FP = 70/30 file-split over-fire;
catch = popularity-weighted coverage. Usage: python benchmarks/arch_graph_rank.py
"""
import random, sys
import numpy as np
from collections import Counter
from arch_graph_probe import corpus_edges, CORPORA

random.seed(0)
DELTAS = [1.01, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2]


def ranks_of(W):
    in_m, out_m = Counter(), Counter()
    for (a, b), c in W.items():
        out_m[a] += c
        in_m[b] += c
    layers = {l for e in W for l in e}
    return {l: (in_m[l] / (in_m[l] + out_m[l]) if (in_m[l] + out_m[l]) else 0.0)
            for l in layers}, in_m


def fires(a, b, W, rk, delta):
    if (a, b) in W:
        return False  # attested — not novel
    if (b, a) in W:
        return True   # reversal (kept as a strong discrete tell)
    return rk.get(a, 0.0) - rk.get(b, 0.0) >= delta


def main():
    # precompute per corpus: full graph W, per-corpus 70/30 fit graph
    data = {}
    for corp in CORPORA:
        pf = corpus_edges(corp)
        if not pf:
            continue
        W = Counter()
        for edges in pf:
            for e in edges:
                W[e] += 1
        files = pf[:]
        random.shuffle(files)
        ntr = int(0.7 * len(files))
        Wfit = Counter()
        for edges in files[:ntr]:
            for e in edges:
                Wfit[e] += 1
        data[corp] = (W, files, ntr, Wfit)

    print(f"{'delta':>6} {'meanFP%':>8} {'worstFP%':>9} {'meanCatch%':>11} "
          f"{'minCatch%':>10}")
    print("-" * 48)
    for delta in DELTAS:
        fps, catches = [], []
        for corp, (W, files, ntr, Wfit) in data.items():
            rk_fit, _ = ranks_of(Wfit)
            # FP: held-out files' novel edges that fire vs the fit graph
            novel = fire = 0
            seen = set()
            for edges in files[ntr:]:
                for (a, b) in edges:
                    if (a, b) in Wfit:
                        continue
                    novel += 1
                    if fires(a, b, Wfit, rk_fit, delta):
                        fire += 1
            fp = 100 * fire / novel if novel else 0
            fps.append(fp)
            # catch: popularity-weighted coverage over plausible missing edges
            rk, in_m = ranks_of(W)
            layers = list({l for e in W for l in e})
            num = den = 0.0
            for a in layers:
                for b in layers:
                    if a != b and (a, b) not in W and in_m[b] > 0:
                        den += in_m[b]
                        if fires(a, b, W, rk, delta):
                            num += in_m[b]
            catches.append(100 * num / den if den else 0)
        print(f"{delta:6.2f} {np.mean(fps):7.1f}% {max(fps):8.1f}% "
              f"{np.mean(catches):10.0f}% {min(catches):9.0f}%")
    print("\nFP = file-split over-fire of the rule; catch = popularity-weighted "
          "coverage. Want meanCatch>=85 with worstFP<=5. (delta=1.01 ≈ reversal∪sink.)")


if __name__ == "__main__":
    main()
