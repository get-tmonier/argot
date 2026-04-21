# engine/argot/research/signal/phase14/scorers/sequential_import_bpe_scorer.py
"""Sequential import-graph → BPE-tfidf scorer.

Stage 1: ImportGraphScorer — if score ≥ 1, flag immediately (foreign module found).
Stage 2: BPE-tfidf — for hunks where Stage 1 returned 0, flag if BPE score exceeds
         per-repo threshold (= max BPE score over calibration hunks).

Both scores are always computed so callers can use the full trace for diagnostics.
"""

from __future__ import annotations

import json
import math
from collections import Counter
from collections.abc import Iterable
from pathlib import Path
from typing import Any, Literal

from argot.research.signal.phase14.scorers.import_graph_scorer import ImportGraphScorer

_EPSILON = 1e-7
_BPE_MODEL_NAME = "microsoft/unixcoder-base"


def _is_meaningful_token(token_str: str) -> bool:
    return len(token_str) >= 3 and any(c.isalnum() for c in token_str)


ScoredHunk = dict[str, Any]
Reason = Literal["import", "bpe", "none"]


class SequentialImportBpeScorer:
    """Two-stage scorer: import-graph fast path, then BPE-tfidf residual.

    Args:
        model_a_files: Source files of the repo being analysed (model_A corpus).
        bpe_model_b_path: Path to generic_tokens_bpe.json (model_B reference).
        calibration_hunks: Representative normal hunks from the target repo.
            BPE threshold is set to max(bpe_score(h) for h in calibration_hunks).
        _tokenizer: Optional pre-loaded tokenizer; loads UnixCoder if None (for DI in tests).
    """

    def __init__(
        self,
        model_a_files: Iterable[Path],
        bpe_model_b_path: Path,
        calibration_hunks: list[str],
        *,
        _tokenizer: Any = None,
    ) -> None:
        model_a_list = list(model_a_files)

        # Stage 1: import-graph scorer
        self._import_scorer = ImportGraphScorer()
        self._import_scorer.fit(model_a_list)

        # BPE tokenizer
        if _tokenizer is None:
            from transformers import AutoTokenizer

            _tokenizer = AutoTokenizer.from_pretrained(_BPE_MODEL_NAME)  # type: ignore[no-untyped-call]
        self._tokenizer = _tokenizer
        vocab: dict[str, int] = _tokenizer.get_vocab()
        self._id_to_token: dict[int, str] = {v: k for k, v in vocab.items()}

        # BPE model B (generic reference corpus)
        raw: dict[str, Any] = json.loads(bpe_model_b_path.read_text(encoding="utf-8"))
        self._model_b: dict[int, int] = {int(k): v for k, v in raw["token_counts"].items()}
        self._total_b: int = raw["total_tokens"]

        # BPE model A (per-repo corpus)
        counts: Counter[int] = Counter()
        for path in model_a_list:
            try:
                source = path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            ids: list[int] = _tokenizer.encode(source, add_special_tokens=False)
            counts.update(ids)
        self._model_a: dict[int, int] = dict(counts)
        self._total_a: int = sum(counts.values()) or 1  # avoid division by zero

        # Per-repo BPE threshold: max score over calibration hunks
        cal_scores = [self._bpe_score(h) for h in calibration_hunks]
        self.bpe_threshold: float = max(cal_scores) if cal_scores else 0.0
        self.n_calibration: int = len(cal_scores)

    def _bpe_score(self, hunk_source: str) -> float:
        ids: list[int] = self._tokenizer.encode(hunk_source, add_special_tokens=False)
        filtered = [i for i in ids if _is_meaningful_token(self._id_to_token.get(i, ""))]
        if not filtered:
            filtered = ids
        if not filtered:
            return 0.0
        scores = [
            math.log(self._model_b.get(i, 0) / self._total_b + _EPSILON)
            - math.log(self._model_a.get(i, 0) / self._total_a + _EPSILON)
            for i in filtered
        ]
        return max(scores)

    def score_hunk(self, hunk_source: str) -> ScoredHunk:
        """Score a hunk through both stages.

        Returns a dict with keys:
          - import_score (float): number of foreign modules (Stage 1 output)
          - bpe_score (float): max log-likelihood ratio (Stage 2 output, always computed)
          - flagged (bool): True if either stage fires
          - reason ("import" | "bpe" | "none"): which stage fired first
        """
        import_score: float = self._import_scorer.score_hunk(hunk_source)
        bpe_score: float = self._bpe_score(hunk_source)

        reason: Reason
        if import_score >= 1.0:
            reason = "import"
        elif bpe_score > self.bpe_threshold:
            reason = "bpe"
        else:
            reason = "none"

        return {
            "import_score": import_score,
            "bpe_score": bpe_score,
            "flagged": reason != "none",
            "reason": reason,
        }
