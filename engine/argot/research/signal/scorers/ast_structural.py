"""AST structural frequency scorer — three variants.

Builds Laplace-smoothed frequency tables over fully corpus-derived AST feature
categories, then scores fixtures by how surprising their structural fingerprint
is relative to those tables.

Three variants:
  loglik  — sum of -log P(v|c) across all (category, value) pairs
  zscore  — per-category z-score of log-prob, summed across categories
  oov     — count of (category, value) pairs unseen in the corpus
"""

from __future__ import annotations

import math
import statistics
from typing import Any, Literal

from argot.research.signal.ast_features import extract_features
from argot.research.signal.base import REGISTRY


def _source_for_record(record: dict[str, Any]) -> str:
    """Return best-effort Python source from a record dict."""
    src_lines = record.get("_source_lines")
    if src_lines is not None:
        return "\n".join(src_lines)
    tokens = record.get("hunk_tokens", [])
    return "".join(t["text"] for t in tokens)


class AstStructuralScorer:
    name = "ast_structural"

    def __init__(
        self,
        *,
        variant: Literal["loglik", "zscore", "oov"],
        alpha: float = 1.0,
        parent_context: bool = False,
        cooccurrence: bool = False,
    ) -> None:
        self._variant = variant
        self._alpha = alpha
        self._parent_context = parent_context
        self._cooccurrence = cooccurrence

        # Populated by fit():
        # _counts[c][v] = raw occurrence count
        self._counts: dict[str, dict[str, int]] = {}
        # _N[c] = total tokens observed in category c
        self._N: dict[str, int] = {}
        # _V[c] = vocabulary size in category c
        self._V: dict[str, int] = {}
        # For zscore: per-category (mean, std) of corpus chunk log-probs
        self._cat_stats: dict[str, tuple[float, float]] = {}

    # ------------------------------------------------------------------
    # Fit
    # ------------------------------------------------------------------

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        counts: dict[str, dict[str, int]] = {}
        totals: dict[str, int] = {}

        for record in corpus:
            src = _source_for_record(record)
            feats = extract_features(
                src, parent_context=self._parent_context, cooccurrence=self._cooccurrence
            )
            for cat, values in feats.items():
                if cat not in counts:
                    counts[cat] = {}
                    totals[cat] = 0
                for v in values:
                    counts[cat][v] = counts[cat].get(v, 0) + 1
                    totals[cat] += 1

        self._counts = counts
        self._N = totals
        self._V = {cat: len(vocab) for cat, vocab in counts.items()}

        if self._variant == "zscore":
            self._fit_cat_stats(corpus)

    def _fit_cat_stats(self, corpus: list[dict[str, Any]]) -> None:
        """Precompute per-category log-prob mean/std across corpus chunks."""
        cat_logprobs: dict[str, list[float]] = {}
        for record in corpus:
            src = _source_for_record(record)
            feats = extract_features(
                src, parent_context=self._parent_context, cooccurrence=self._cooccurrence
            )
            for cat, values in feats.items():
                lp = sum(-math.log(self._prob(cat, v)) for v in values)
                cat_logprobs.setdefault(cat, []).append(lp)

        for cat, lps in cat_logprobs.items():
            mu = statistics.mean(lps) if lps else 0.0
            sigma = statistics.stdev(lps) if len(lps) >= 2 else 1.0
            if sigma == 0.0:
                sigma = 1.0
            self._cat_stats[cat] = (mu, sigma)

    # ------------------------------------------------------------------
    # Scoring helpers
    # ------------------------------------------------------------------

    def _prob(self, cat: str, v: str) -> float:
        """Laplace-smoothed P(v | cat)."""
        alpha = self._alpha
        count_cv = self._counts.get(cat, {}).get(v, 0)
        n_c = self._N.get(cat, 0)
        v_c = self._V.get(cat, 0)
        return (count_cv + alpha) / (n_c + alpha * (v_c + 1))

    def _score_loglik(self, feats: dict[str, list[str]]) -> float:
        total = 0.0
        for cat, values in feats.items():
            for v in values:
                total += -math.log(self._prob(cat, v))
        return total

    def _score_zscore(self, feats: dict[str, list[str]]) -> float:
        total = 0.0
        for cat, values in feats.items():
            lp = sum(-math.log(self._prob(cat, v)) for v in values)
            mu, sigma = self._cat_stats.get(cat, (0.0, 1.0))
            total += (lp - mu) / sigma
        return total

    def _score_oov(self, feats: dict[str, list[str]]) -> float:
        total = 0.0
        for cat, values in feats.items():
            cat_counts = self._counts.get(cat, {})
            for v in values:
                if cat_counts.get(v, 0) == 0:
                    total += 1.0
        return total

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        results: list[float] = []
        for record in fixtures:
            src = _source_for_record(record)
            feats = extract_features(
                src, parent_context=self._parent_context, cooccurrence=self._cooccurrence
            )
            if self._variant == "loglik":
                results.append(self._score_loglik(feats))
            elif self._variant == "zscore":
                results.append(self._score_zscore(feats))
            else:
                results.append(self._score_oov(feats))
        return results


REGISTRY["ast_structural_ll"] = lambda: AstStructuralScorer(variant="loglik")
REGISTRY["ast_structural_zscore"] = lambda: AstStructuralScorer(variant="zscore")
REGISTRY["ast_structural_oov"] = lambda: AstStructuralScorer(variant="oov")
REGISTRY["ast_structural_ctx_ll"] = lambda: AstStructuralScorer(
    variant="loglik", parent_context=True
)
REGISTRY["ast_structural_ctx_zscore"] = lambda: AstStructuralScorer(
    variant="zscore", parent_context=True
)
REGISTRY["ast_structural_ctx_oov"] = lambda: AstStructuralScorer(variant="oov", parent_context=True)
REGISTRY["ast_structural_cooc_ll"] = lambda: AstStructuralScorer(
    variant="loglik", cooccurrence=True
)
REGISTRY["ast_structural_cooc_zscore"] = lambda: AstStructuralScorer(
    variant="zscore", cooccurrence=True
)
REGISTRY["ast_structural_cooc_oov"] = lambda: AstStructuralScorer(variant="oov", cooccurrence=True)
REGISTRY["ast_structural_full_oov"] = lambda: AstStructuralScorer(
    variant="oov", parent_context=True, cooccurrence=True
)
