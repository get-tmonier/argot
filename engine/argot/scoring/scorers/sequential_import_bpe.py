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

from argot.scoring.adapters.language_adapter import LanguageAdapter
from argot.scoring.adapters.registry import adapter_for_files
from argot.scoring.filters.typicality import TypicalityModel, language_for_adapter
from argot.scoring.scorers.call_receiver import CallReceiverScorer
from argot.scoring.scorers.import_graph import ImportGraphScorer

_EPSILON = 1e-7
_BPE_MODEL_NAME = "microsoft/unixcoder-base"

# Matches lines starting with "import " or "from " (no leading spaces — top-of-file only)
_RE_IMPORT_LINE = re.compile(r"^(?:import |from )\S", re.MULTILINE)


def _blank_prose_lines(src: str, ranges: frozenset[int]) -> str:
    """Return *src* with every 1-indexed line number in *ranges* replaced by an empty string.

    Used to suppress prose (docstrings, comments) before BPE scoring so that
    natural-language tokens don't inflate the BPE score.
    """
    if not ranges:
        return src
    lines = src.splitlines(keepends=True)
    result: list[str] = []
    for i, line in enumerate(lines, start=1):
        if i in ranges:
            # Preserve the trailing newline (if any) so line count is stable
            result.append("\n" if line.endswith("\n") else "")
        else:
            result.append(line)
    return "".join(result)


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
        if isinstance(node, ast.Import | ast.ImportFrom):
            end = node.end_lineno if node.end_lineno is not None else node.lineno
            collected.extend(source_lines[node.lineno - 1 : end])
    return "\n".join(collected)


def _is_meaningful_token(token_str: str) -> bool:
    return len(token_str) >= 3 and any(c.isalnum() for c in token_str)


def _compute_threshold(
    cal_scores: list[float],
    threshold_percentile: float | None,
    threshold_iqr_k: float | None = None,
) -> float:
    """Compute BPE threshold from calibration scores.

    Priority: IQR-margin > percentile > max.
    threshold_iqr_k not None → p75 + k * IQR (IQR = p75 - p25), linear interpolation.
    threshold_percentile not None → that percentile via linear interpolation.
    Both None → max(cal_scores).
    """
    if not cal_scores:
        return 0.0
    sorted_vals = sorted(cal_scores)
    n = len(sorted_vals)
    if threshold_iqr_k is not None:
        idx25 = 0.25 * (n - 1)
        lo25 = int(idx25)
        hi25 = min(lo25 + 1, n - 1)
        p25 = sorted_vals[lo25] + (idx25 - lo25) * (sorted_vals[hi25] - sorted_vals[lo25])
        idx75 = 0.75 * (n - 1)
        lo75 = int(idx75)
        hi75 = min(lo75 + 1, n - 1)
        p75 = sorted_vals[lo75] + (idx75 - lo75) * (sorted_vals[hi75] - sorted_vals[lo75])
        return p75 + threshold_iqr_k * (p75 - p25)
    if threshold_percentile is None:
        return max(cal_scores)
    idx = (threshold_percentile / 100.0) * (n - 1)
    lo = int(idx)
    hi = min(lo + 1, n - 1)
    return sorted_vals[lo] + (idx - lo) * (sorted_vals[hi] - sorted_vals[lo])


ScoredHunk = dict[str, Any]
Reason = Literal[
    "import", "call_receiver", "bpe", "none", "auto_generated", "atypical", "atypical_file"
]


class SequentialImportBpeScorer:
    """Two-stage scorer: import-graph fast path, then BPE-tfidf residual.

    Args:
        model_a_files: Source files of the repo being analysed (model_A corpus).
        bpe_model_b_path: Path to generic_tokens_bpe.json (model_B reference).
        calibration_hunks: Representative normal hunks from the target repo.
            BPE threshold is set to max(bpe_score(h) for h in calibration_hunks) when
            threshold_percentile is None, or to that percentile otherwise.
        threshold_percentile: None → max(cal_scores). A value in (0, 100] → that
            percentile of cal_scores via linear interpolation. Default 95.0 (p95) —
            more robust than max to single high-scoring calibration outliers.
        threshold_iqr_k: When not None, overrides threshold_percentile; sets threshold to
            p75 + k * IQR (IQR = p75 - p25). Default None.
        enable_typicality_filter: Build a TypicalityModel for calibration pool filtering
            and inference short-circuit (hunk- and file-level).  Default True.
            Does NOT affect model-A filtering; model A always uses ``exclude_data_dominant``.
        exclude_data_dominant: Filter model-A files using LanguageAdapter.is_data_dominant().
            Unchanged from era 4; typicality does not replace this.
        _tokenizer: Optional pre-loaded tokenizer; loads UnixCoder if None (for DI in tests).
    """

    def __init__(
        self,
        model_a_files: Iterable[Path],
        bpe_model_b_path: Path,
        calibration_hunks: list[str] | None = None,
        *,
        bpe_threshold: float | None = None,
        adapter: LanguageAdapter | None = None,
        repo_root: Path | None = None,
        threshold_percentile: float | None = 95.0,
        threshold_iqr_k: float | None = None,
        exclude_data_dominant: bool = True,
        enable_typicality_filter: bool = True,
        call_receiver_alpha: float = 2.0,
        call_receiver_cap: int = 5,
        _tokenizer: Any = None,
    ) -> None:
        if calibration_hunks is None and bpe_threshold is None:
            raise ValueError("Either calibration_hunks or bpe_threshold must be provided")
        model_a_list = list(model_a_files)

        # Resolve language adapter from file extensions (or use provided adapter)
        self._adapter: LanguageAdapter = (
            adapter if adapter is not None else adapter_for_files([str(p) for p in model_a_list])
        )

        # Typicality model — stateless, language-parameterized.
        self._typicality_model: TypicalityModel | None = None
        if enable_typicality_filter:
            self._typicality_model = TypicalityModel(language=language_for_adapter(self._adapter))

        if exclude_data_dominant:
            filtered: list[Path] = []
            for p in model_a_list:
                try:
                    src = p.read_text(encoding="utf-8", errors="replace")
                except OSError:
                    filtered.append(p)
                    continue
                if not self._adapter.is_data_dominant(src):
                    filtered.append(p)
            if not filtered:
                raise ValueError(
                    f"exclude_data_dominant=True removed all {len(model_a_list)} "
                    "model A file(s); cannot train on an empty corpus."
                )
            model_a_list = filtered

        # Stage 1: import-graph scorer (uses same adapter)
        self._import_scorer = ImportGraphScorer(adapter=self._adapter, repo_root=repo_root)
        self._import_scorer.fit(model_a_list)

        # Stage 1.5: call-receiver soft-penalty scorer
        self._call_receiver: CallReceiverScorer | None = None
        if call_receiver_alpha > 0.0:
            self._call_receiver = CallReceiverScorer(
                model_a_list,
                language=language_for_adapter(self._adapter),
                alpha=call_receiver_alpha,
                cap=call_receiver_cap,
                adapter=self._adapter,
            )

        # BPE tokenizer
        if _tokenizer is None:
            from transformers import AutoTokenizer

            _tokenizer = AutoTokenizer.from_pretrained(_BPE_MODEL_NAME)
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

        if bpe_threshold is not None:
            # Use pre-computed threshold (e.g. loaded from scorer-config.json)
            self.bpe_threshold: float = bpe_threshold
            self.cal_scores: list[float] = []
            self.n_calibration: int = 0
        else:
            cal_list = list(calibration_hunks or [])
            if self._typicality_model is not None:
                cal_list = [h for h in cal_list if not self._typicality_model.is_atypical(h)[0]]
            cal_scores = [
                self._bpe_score(_blank_prose_lines(h, self._adapter.prose_line_ranges(h)))
                for h in cal_list
            ]
            self.cal_scores = cal_scores
            self.bpe_threshold = _compute_threshold(
                cal_scores, threshold_percentile, threshold_iqr_k
            )
            self.n_calibration = len(cal_scores)

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

    def score_hunk(
        self,
        hunk_content: str,
        *,
        file_source: str | None = None,
        hunk_start_line: int | None = None,
        hunk_end_line: int | None = None,
    ) -> ScoredHunk:
        """Score a hunk through both stages.

        Args:
            hunk_content: The raw hunk diff / function body to score.
            file_source: Optional full source of the file containing the hunk.
                When provided, Stage 1 fires only on imports added in the hunk
                itself (not the file's header imports).  A pure string or comment
                edit in an import-heavy file will not trigger Stage 1 because the
                hunk has no import statements of its own.
                Stage 2 always scores ``hunk_content`` only, regardless of
                file_source, to avoid token-position false positives from a
                large file prefix.
            hunk_start_line: 1-indexed line number of the first line of the hunk
                within *file_source*.  Must be provided together with
                *file_source* and *hunk_end_line* to enable prose masking.
            hunk_end_line: 1-indexed line number of the last line of the hunk
                within *file_source* (inclusive).  Must be provided together
                with *file_source* and *hunk_start_line* to enable prose
                masking.

        When *file_source*, *hunk_start_line*, and *hunk_end_line* are all
        provided, Stage 2 blanks any prose lines (docstrings, comments) that
        fall within the hunk range before BPE scoring, mirroring the symmetric
        treatment applied to calibration hunks.

        Returns a dict with keys:
          - import_score (float): number of foreign modules (Stage 1 output)
          - bpe_score (float): max log-likelihood ratio (Stage 2 output, always computed)
          - flagged (bool): True if either stage fires
          - reason ("import" | "bpe" | "none"): which stage fired first
        """
        # Typicality short-circuits (replaces the legacy auto-generated gate).
        if self._typicality_model is not None:
            is_atyp_hunk, _ = self._typicality_model.is_atypical(hunk_content)
            if is_atyp_hunk:
                return {
                    "import_score": 0.0,
                    "bpe_score": 0.0,
                    "flagged": False,
                    "reason": "atypical",
                }
            if file_source is not None:
                is_atyp_file, _ = self._typicality_model.is_atypical_file(file_source)
                if is_atyp_file:
                    return {
                        "import_score": 0.0,
                        "bpe_score": 0.0,
                        "flagged": False,
                        "reason": "atypical_file",
                    }
        elif file_source is not None and self._adapter.is_auto_generated(file_source):
            return {
                "import_score": 0.0,
                "bpe_score": 0.0,
                "flagged": False,
                "reason": "auto_generated",
            }

        if file_source is not None:
            # Stage 1 — hunk-only: only imports added in the hunk can introduce a
            # foreign module.  A pure string/comment edit in an import-heavy file
            # has no hunk imports and cannot trigger Stage 1.
            hunk_imports = self._adapter.extract_imports(hunk_content)
            foreign = {spec for spec in hunk_imports if self._import_scorer.is_foreign(spec)}
            import_score: float = float(len(foreign))
        else:
            import_score = self._import_scorer.score_hunk(hunk_content)

        # Stage 2: optionally blank prose lines before BPE scoring
        bpe_input = hunk_content
        if file_source is not None and hunk_start_line is not None and hunk_end_line is not None:
            file_prose = self._adapter.prose_line_ranges(file_source)
            # Intersect global prose ranges with the hunk's line span, then
            # re-index to 1-based lines within hunk_content itself.
            hunk_prose_local: frozenset[int] = frozenset(
                ln - hunk_start_line + 1
                for ln in file_prose
                if hunk_start_line <= ln <= hunk_end_line
            )
            bpe_input = _blank_prose_lines(hunk_content, hunk_prose_local)

        bpe_score: float = self._bpe_score(bpe_input)

        if import_score >= 1.0:
            return {
                "import_score": import_score,
                "bpe_score": bpe_score,
                "flagged": True,
                "reason": "import",
            }

        # Stage 1.5: call-receiver soft penalty
        if self._call_receiver is not None:
            n_unattested = self._call_receiver.count_unattested(hunk_content)
            adjusted_bpe = bpe_score + self._call_receiver.alpha * min(
                n_unattested, self._call_receiver.cap
            )
            if adjusted_bpe > self.bpe_threshold:
                cr_reason: Reason = "call_receiver" if bpe_score <= self.bpe_threshold else "bpe"
                return {
                    "import_score": import_score,
                    "bpe_score": bpe_score,
                    "flagged": True,
                    "reason": cr_reason,
                }
            return {
                "import_score": import_score,
                "bpe_score": bpe_score,
                "flagged": False,
                "reason": "none",
            }

        reason: Reason = "bpe" if bpe_score > self.bpe_threshold else "none"
        return {
            "import_score": import_score,
            "bpe_score": bpe_score,
            "flagged": reason != "none",
            "reason": reason,
        }
