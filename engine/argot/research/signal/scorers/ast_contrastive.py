from __future__ import annotations

import json
import math
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from argot.research.signal.base import REGISTRY
from argot.research.signal.treelet_extractor import extract_treelets

_REFERENCE_PATH = Path(__file__).parent.parent.parent / "reference" / "generic_treelets.json"


def _load_reference() -> Counter[str]:
    data = json.loads(_REFERENCE_PATH.read_text())
    return Counter(data["treelet_counts"])


def _record_to_source(record: dict[str, Any]) -> str:
    """Extract the best available Python source string from a record.

    Priority order:
    1. ``hunk_source`` — exact source lines added to fixture records by
       :func:`~argot.acceptance.runner.fixture_to_record`.
    2. ``start_line``-grouped token reconstruction — used for corpus records
       whose tokens carry per-token line numbers.
    3. Space-joined token texts — last-resort fallback.
    """
    if "hunk_source" in record:
        return record["hunk_source"]  # type: ignore[no-any-return]
    hunk_tokens = record["hunk_tokens"]
    if hunk_tokens and "start_line" in hunk_tokens[0]:
        lines_map: dict[int, list[str]] = defaultdict(list)
        for tok in hunk_tokens:
            lines_map[tok["start_line"]].append(tok["text"])
        return "\n".join(" ".join(lines_map[k]) for k in sorted(lines_map))
    return " ".join(t["text"] for t in hunk_tokens)


class ContrastiveAstTreeletScorer:
    name = "ast_contrastive"

    def __init__(self, *, epsilon: float = 1.0) -> None:
        self._epsilon = epsilon
        self._model_a: Counter[str] = Counter()
        self._model_b: Counter[str] = _load_reference()

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        counts: Counter[str] = Counter()
        for record in corpus:
            source = _record_to_source(record)
            counts.update(extract_treelets(source))
        self._model_a = counts

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        """Score each fixture.

        Returns an **anomaly** score: higher means the hunk's AST treelets are
        *less* like the repo corpus (model_A) relative to generic Python
        (model_B).  A paradigm break should contain constructs that are rare in
        the repo but common generically, yielding a high anomaly score.

        Formula per treelet *t*:
            ``log(model_B[t] + ε) - log(model_A[t] + ε)``
        """
        eps = self._epsilon
        results: list[float] = []
        for record in fixtures:
            hunk_source = _record_to_source(record)
            treelets = extract_treelets(hunk_source)
            if len(treelets) < 3:
                results.append(0.0)
                continue
            total = sum(
                math.log(self._model_b[t] + eps) - math.log(self._model_a[t] + eps)
                for t in treelets
            )
            results.append(total / len(treelets))
        return results


REGISTRY["ast_contrastive"] = lambda: ContrastiveAstTreeletScorer()
REGISTRY["ast_contrastive_e01"] = lambda: ContrastiveAstTreeletScorer(epsilon=0.1)
REGISTRY["ast_contrastive_e10"] = lambda: ContrastiveAstTreeletScorer(epsilon=10.0)
