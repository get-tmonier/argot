from __future__ import annotations

import json
import math
from collections import Counter
from pathlib import Path
from typing import Any

from argot.research.signal.base import REGISTRY
from argot.research.signal.treelet_extractor import extract_treelets

_REFERENCE_PATH = Path(__file__).parent.parent.parent / "reference" / "generic_treelets.json"


def _load_reference() -> Counter[str]:
    data = json.loads(_REFERENCE_PATH.read_text())
    return Counter(data["treelet_counts"])


class ContrastiveAstTreeletScorer:
    name = "ast_contrastive"

    def __init__(self, *, epsilon: float = 1.0) -> None:
        self._epsilon = epsilon
        self._model_a: Counter[str] = Counter()
        self._model_b: Counter[str] = _load_reference()

    def fit(self, corpus: list[dict[str, Any]]) -> None:
        seen: set[str] = set()
        counts: Counter[str] = Counter()
        for record in corpus:
            path: str = record["file_path"]
            if path in seen:
                continue
            seen.add(path)
            all_tokens = list(record["context_before"]) + list(record["hunk_tokens"])
            source = " ".join(t["text"] for t in all_tokens)
            treelets = extract_treelets(source)
            counts.update(treelets)
        self._model_a = counts

    def score(self, fixtures: list[dict[str, Any]]) -> list[float]:
        eps = self._epsilon
        results: list[float] = []
        for record in fixtures:
            hunk_source = " ".join(t["text"] for t in record["hunk_tokens"])
            treelets = extract_treelets(hunk_source)
            if len(treelets) < 3:
                results.append(0.0)
                continue
            total = sum(
                math.log(self._model_a[t] + eps) - math.log(self._model_b[t] + eps)
                for t in treelets
            )
            results.append(total / len(treelets))
        return results


REGISTRY["ast_contrastive"] = lambda: ContrastiveAstTreeletScorer()
REGISTRY["ast_contrastive_e01"] = lambda: ContrastiveAstTreeletScorer(epsilon=0.1)
REGISTRY["ast_contrastive_e10"] = lambda: ContrastiveAstTreeletScorer(epsilon=10.0)
