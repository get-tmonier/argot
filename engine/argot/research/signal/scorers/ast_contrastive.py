from __future__ import annotations

import contextlib
import json
import math
from collections import Counter, defaultdict
from collections.abc import Iterable
from pathlib import Path
from typing import Any

from argot.research.signal.base import REGISTRY
from argot.research.signal.treelet_extractor import extract_treelets

_REFERENCE_PATH = Path(__file__).parent.parent.parent / "reference" / "generic_treelets.json"


def _load_reference() -> Counter[str]:
    data = json.loads(_REFERENCE_PATH.read_text())
    return Counter(data["treelet_counts"])


def _total(counter: Counter[str]) -> int:
    return sum(counter.values()) or 1


def _record_to_source(record: dict[str, Any]) -> str:
    """Extract the best available Python source string from a record.

    Priority order:
    1. ``hunk_source`` — exact source lines from fixture records (lossless).
    2. ``start_line``-grouped token reconstruction — lossy approximation for
       corpus records; tokens are re-joined per line without original spacing.
    3. Space-joined token texts — last-resort lossy fallback.
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

    def __init__(self, *, epsilon: float = 1.0, aggregation: str = "mean") -> None:
        if aggregation not in {"mean", "max"}:
            raise ValueError(f"aggregation must be 'mean' or 'max', got {aggregation!r}")
        self._epsilon = epsilon
        self._aggregation = aggregation
        self._model_a: Counter[str] = Counter()
        self._model_b: Counter[str] = _load_reference()

    @property
    def _total_a(self) -> int:
        return _total(self._model_a)

    @property
    def _total_b(self) -> int:
        return _total(self._model_b)

    def fit(
        self,
        corpus: list[dict[str, Any]],
        *,
        model_a_files: Iterable[Path] | None = None,
    ) -> None:
        """Build model_A from full Python files when available, else from corpus records.

        ``model_a_files`` should be complete ``.py`` files (100 % parse rate).
        The corpus-record fallback has ~14 % parse rate because git hunks are
        fragments, so it produces a very sparse model_A that kills the contrast.
        """
        counts: Counter[str] = Counter()
        if model_a_files is not None:
            for path in model_a_files:
                with contextlib.suppress(Exception):
                    counts.update(extract_treelets(path.read_text(errors="replace")))
        else:
            for record in corpus:
                source = _record_to_source(record)
                counts.update(extract_treelets(source))
        self._model_a = counts

    def _treelets_for_record(self, record: dict[str, Any]) -> list[str]:
        """Extract treelets, falling back to the full fixture file when the
        hunk slice is a mid-block fragment (SyntaxError on ast.parse)."""
        treelets = extract_treelets(_record_to_source(record))
        if len(treelets) >= 3:
            return treelets
        # Fallback: parse the full fixture file if its path was recorded.
        # Mid-block hunks (e.g., code inside a class body) are unparseable as
        # standalone modules but the whole file always is.
        fixture_path = record.get("_fixture_path")
        if fixture_path:
            treelets = extract_treelets(Path(fixture_path).read_text(errors="replace"))
        return treelets

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
        ta = self._total_a
        tb = self._total_b
        results: list[float] = []
        for record in fixtures:
            treelets = self._treelets_for_record(record)
            if len(treelets) < 3:
                results.append(0.0)
                continue
            # Frequency-normalised log-ratio: compare treelet *rates* so the
            # 800× size difference between model_B (CPython, 3.2M) and model_A
            # (20 control files, ~4K) does not inflate every score.
            per_treelet = [
                math.log(self._model_b[t] / tb + eps) - math.log(self._model_a[t] / ta + eps)
                for t in treelets
            ]
            if self._aggregation == "max":
                results.append(max(per_treelet))
            else:
                results.append(sum(per_treelet) / len(per_treelet))
        return results


REGISTRY["ast_contrastive"] = lambda: ContrastiveAstTreeletScorer(epsilon=1e-6)
REGISTRY["ast_contrastive_e01"] = lambda: ContrastiveAstTreeletScorer(epsilon=1e-5)
REGISTRY["ast_contrastive_e10"] = lambda: ContrastiveAstTreeletScorer(epsilon=1e-7)
REGISTRY["ast_contrastive_max"] = lambda: ContrastiveAstTreeletScorer(
    epsilon=1e-7, aggregation="max"
)
