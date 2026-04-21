from __future__ import annotations

import random


def auc_from_scores(break_scores: list[float], ctrl_scores: list[float]) -> float:
    """Mann-Whitney AUC estimate."""
    n_break = len(break_scores)
    n_ctrl = len(ctrl_scores)
    if n_break == 0 or n_ctrl == 0:
        return 0.5
    wins = sum(1 for b in break_scores for c in ctrl_scores if b > c)
    ties = sum(1 for b in break_scores for c in ctrl_scores if b == c)
    return (wins + 0.5 * ties) / (n_break * n_ctrl)


def paired_bootstrap_ci(
    baseline_break: list[float],
    baseline_ctrl: list[float],
    variant_break: list[float],
    variant_ctrl: list[float],
    n_resamples: int = 1000,
    ci: float = 0.95,
    seed: int = 42,
) -> tuple[float, float, float]:
    """Returns (delta, ci_low, ci_high) where delta = AUC(variant) - AUC(baseline)."""
    rng = random.Random(seed)
    n_break = len(baseline_break)
    n_ctrl = len(baseline_ctrl)

    base_auc = auc_from_scores(baseline_break, baseline_ctrl)
    var_auc = auc_from_scores(variant_break, variant_ctrl)
    delta = var_auc - base_auc

    boot_deltas: list[float] = []
    for _ in range(n_resamples):
        b_idx = [rng.randrange(n_break) for _ in range(n_break)]
        c_idx = [rng.randrange(n_ctrl) for _ in range(n_ctrl)]
        b_base = auc_from_scores(
            [baseline_break[i] for i in b_idx], [baseline_ctrl[i] for i in c_idx]
        )
        b_var = auc_from_scores(
            [variant_break[i] for i in b_idx], [variant_ctrl[i] for i in c_idx]
        )
        boot_deltas.append(b_var - b_base)

    boot_deltas.sort()
    alpha = (1 - ci) / 2
    lo = boot_deltas[int(alpha * n_resamples)]
    hi = boot_deltas[int((1 - alpha) * n_resamples)]
    return delta, lo, hi


__all__ = ["auc_from_scores", "paired_bootstrap_ci"]
