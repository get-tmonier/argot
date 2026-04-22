# engine/argot/research/signal/phase14/scorers/sequential_import_bpe_scorer.py
"""Sequential import-graph → BPE-tfidf scorer.

Stage 1: ImportGraphScorer — if score ≥ 1, flag immediately (foreign module found).
Stage 2: BPE-tfidf — for hunks where Stage 1 returned 0, flag if BPE score exceeds
         per-repo threshold (= max BPE score over calibration hunks).

Both scores are always computed so callers can use the full trace for diagnostics.
"""

from __future__ import annotations

import ast
import json
import math
import re
from collections import Counter
from collections.abc import Iterable
from pathlib import Path
from typing import Any, Literal

from argot.research.signal.phase14.scorers.import_graph_scorer import (
    ImportGraphScorer,
    _imports_from_ast,
)

_EPSILON = 1e-7
_BPE_MODEL_NAME = "microsoft/unixcoder-base"

# Matches lines starting with "import " or "from " (no leading spaces — top-of-file only)
_RE_IMPORT_LINE = re.compile(r"^(?:import |from )\S", re.MULTILINE)


def extract_imports(source: str) -> str:
    """Return just the top-of-file import block from *source*.

    Uses ``ast.parse`` where the source is valid Python: collects all
    ``ast.Import`` / ``ast.ImportFrom`` nodes that appear before the first
    non-import top-level statement and returns the corresponding source lines.

    Falls back to a line-prefix regex on ``SyntaxError``: matches lines
    starting with ``import `` or ``from `` (exact keyword + space, at column 0).
    This is deliberately conservative — no indented imports are collected.

    Returns the import lines joined with ``\\n``, or an empty string if none.
    """
    try:
        tree = ast.parse(source)
    except SyntaxError:
        # Regex fallback: only match lines at column 0 with exact keyword prefix
        lines = source.splitlines()
        import_lines = [ln for ln in lines if _RE_IMPORT_LINE.match(ln)]
        return "\n".join(import_lines)

    source_lines = source.splitlines()
    collected: list[str] = []
    for node in ast.iter_child_nodes(tree):
        if isinstance(node, (ast.Import, ast.ImportFrom)):
            end = node.end_lineno if node.end_lineno is not None else node.lineno
            collected.extend(source_lines[node.lineno - 1 : end])
    return "\n".join(collected)


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

    def score_hunk(self, hunk_content: str, *, file_source: str | None = None) -> ScoredHunk:
        """Score a hunk through both stages.

        Args:
            hunk_content: The raw hunk diff / function body to score.
            file_source: Optional full source of the file containing the hunk.
                When provided, Stage 1 detects foreign modules from both the
                file's import block and the hunk itself by parsing each input
                separately and unioning the results.  This avoids passing a
                concatenated (potentially invalid-Python) string to ``ast.parse``
                and removes the need for a regex fallback in
                ``_imports_from_ast``.
                Stage 2 always scores ``hunk_content`` only, regardless of
                file_source, to avoid token-position false positives from a
                large file prefix.

        Returns a dict with keys:
          - import_score (float): number of foreign modules (Stage 1 output)
          - bpe_score (float): max log-likelihood ratio (Stage 2 output, always computed)
          - flagged (bool): True if either stage fires
          - reason ("import" | "bpe" | "none"): which stage fired first
        """
        if file_source is not None:
            # Stage 1 — split: parse the import block and the hunk independently.
            # extract_imports() returns only import lines (always valid Python), so
            # ast.parse succeeds.  The hunk may be a mid-block slice (SyntaxError is
            # fine — _imports_from_ast returns set() in that case).
            file_imports = _imports_from_ast(extract_imports(file_source))
            hunk_imports = _imports_from_ast(hunk_content)
            all_imports = file_imports | hunk_imports
            foreign = all_imports - self._import_scorer._repo_modules
            import_score: float = float(len(foreign))
        else:
            import_score = self._import_scorer.score_hunk(hunk_content)

        bpe_score: float = self._bpe_score(hunk_content)

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
