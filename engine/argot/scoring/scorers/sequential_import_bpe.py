# engine/argot/research/signal/phase14/scorers/sequential_import_bpe_scorer.py
"""Sequential import-graph → BPE-tfidf scorer.

Import checker: ImportGraphScorer — if score ≥ 1, flag immediately (foreign module found).
BPE scorer: BPE-tfidf — for hunks where the import checker returned 0, flag if BPE score
            exceeds per-repo threshold (= max BPE score over calibration hunks).

Both scores are always computed so callers can use the full trace for diagnostics.
"""

from __future__ import annotations

import ast
import json
import math
import re
from collections import Counter
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Any, Literal

from argot.scoring.adapters.language_adapter import LanguageAdapter
from argot.scoring.adapters.registry import adapter_for_files
from argot.scoring.filters.typicality import TypicalityModel, language_for_adapter
from argot.scoring.scorers.call_receiver import CallReceiverScorer
from argot.scoring.scorers.import_graph import ImportGraphScorer
from argot.scoring.scorers.shape_primitive import ShapePrimitive

if TYPE_CHECKING:
    from argot.scoring.evidence.types import Evidence, EvidenceCorpus

# Local imports for evidence collectors live inside ``score_hunk`` to keep
# the static import graph free of evidence-package dependencies — it stays
# possible to use the scorer in code paths (tests, ML feature extraction)
# that never touch the evidence layer without paying its import cost.

_EPSILON = 1e-7
_BPE_MODEL_NAME = "microsoft/unixcoder-base"
# The import stage's threshold is the constant "≥ 1 foreign module" rule.
# Surfaced as a named constant so the multi-reason ratio computation in
# Step 2 has something to divide by without a magic number.
_IMPORT_THRESHOLD: float = 1.0

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


Reason = Literal[
    "import", "call_receiver", "bpe", "none", "auto_generated", "atypical", "atypical_file"
]


@dataclass(frozen=True)
class StageScores:
    """Raw per-stage scores from one ``score_hunk`` invocation.

    Always populated alongside ``ScoredHunk`` so diagnostic callers
    (``ml/features.py``, BPE parity tests, future ``--debug-evidence``
    output) can inspect every stage's raw contribution without having to
    call private methods on the scorer. Short-circuit paths (atypical,
    atypical_file, auto_generated) return zeros: stages did not run.
    """

    import_score: float
    bpe_score: float
    call_receiver_contribution: float


@dataclass(frozen=True)
class ScoredHunk:
    """One hunk's verdict, the winning reason's headline score, plus diagnostics.

    ``score`` and ``threshold`` belong to the *winning* reason — not always
    ``bpe_score``. When the import stage fires, ``score`` is the foreign-module
    count and ``threshold`` is :data:`_IMPORT_THRESHOLD` (1.0); when the BPE
    or call-receiver stage fires, ``score`` is the adjusted BPE score and
    ``threshold`` is the calibrated ``bpe_threshold``. For non-flagged or
    short-circuited hunks the headline values still describe one of the
    stages (so the dataclass remains rectangular and JSON-serialisable),
    but the renderer will skip them via ``flagged``.

    ``evidence`` is populated by per-reason collectors in Step 3 of the
    evidence-layer PRD; it is ``None`` until a collector runs.

    ``stages`` carries the raw per-stage breakdown so multi-reason logic
    and ML feature extraction can inspect every stage in one pass.
    """

    score: float
    threshold: float
    flagged: bool
    reason: Reason
    stages: StageScores
    evidence: Evidence | None = None


class SequentialImportBpeScorer:
    """Two-stage scorer: import checker fast path, then BPE scorer residual.

    Args:
        repo_corpus_files: Source files of the repo being analysed (repo corpus).
        bpe_generic_baseline_path: Path to generic_tokens_bpe.json (generic baseline reference).
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
            Does NOT affect repo corpus filtering; repo corpus always uses
            ``exclude_data_dominant``.
        exclude_data_dominant: Filter repo corpus files using LanguageAdapter.is_data_dominant().
            Operates independently of the typicality filter.
        _tokenizer: Optional pre-loaded tokenizer; loads UnixCoder if None (for DI in tests).
    """

    def __init__(
        self,
        repo_corpus_files: Iterable[Path],
        bpe_generic_baseline_path: Path,
        calibration_hunks: list[str] | None = None,
        *,
        bpe_threshold: float | None = None,
        adapter: LanguageAdapter | None = None,
        repo_root: Path | None = None,
        threshold_percentile: float | None = None,
        threshold_iqr_k: float | None = None,
        exclude_data_dominant: bool = True,
        enable_typicality_filter: bool = True,
        call_receiver_alpha: float = 2.0,
        call_receiver_cap: int = 5,
        call_receiver_root_bonus: float = 2.0,
        call_receiver_n_clusters: int = 8,
        call_receiver_cluster_seed: int = 0,
        call_receiver_cluster_bonus: float = 5.0,
        call_receiver_cluster_rare_threshold: int = 0,
        call_receiver_cluster_size_min: int = 0,
        call_receiver_force_jaccard_routing: bool = False,
        call_receiver_shape_primitives: list[ShapePrimitive[Any]] | None = None,
        calibration_hunks_with_metadata: list[tuple[str, Path, str]] | None = None,
        evidence_corpus: EvidenceCorpus | None = None,
        import_modules_snapshot: tuple[frozenset[str], frozenset[str]] | None = None,
        _tokenizer: Any = None,
    ) -> None:
        if (
            calibration_hunks is None
            and calibration_hunks_with_metadata is None
            and bpe_threshold is None
        ):
            raise ValueError(
                "Either calibration_hunks, calibration_hunks_with_metadata, or "
                "bpe_threshold must be provided"
            )
        repo_corpus_list = list(repo_corpus_files)

        # Resolve language adapter from file extensions (or use provided adapter)
        self._adapter: LanguageAdapter = (
            adapter
            if adapter is not None
            else adapter_for_files([str(p) for p in repo_corpus_list])
        )

        # Typicality model — stateless, language-parameterized.
        self._typicality_model: TypicalityModel | None = None
        if enable_typicality_filter:
            self._typicality_model = TypicalityModel(language=language_for_adapter(self._adapter))

        if exclude_data_dominant:
            filtered: list[Path] = []
            for p in repo_corpus_list:
                try:
                    src = p.read_text(encoding="utf-8", errors="replace")
                except OSError:
                    filtered.append(p)
                    continue
                if not self._adapter.is_data_dominant(src):
                    filtered.append(p)
            if not filtered:
                raise ValueError(
                    f"exclude_data_dominant=True removed all {len(repo_corpus_list)} "
                    "repo corpus file(s); cannot train on an empty corpus."
                )
            repo_corpus_list = filtered

        # Import checker: import-graph scorer (uses same adapter). Prefer a
        # fit-time snapshot when one is supplied — it pins the foreign-set to
        # what the model knew at fit, so a hunk that introduces a brand-new
        # import is still flagged as foreign rather than getting absorbed when
        # the scorer re-derives modules from current file contents.
        self._import_scorer = ImportGraphScorer(adapter=self._adapter, repo_root=repo_root)
        if import_modules_snapshot is not None:
            modules, prefixes = import_modules_snapshot
            self._import_scorer.load_snapshot(modules, prefixes)
        else:
            self._import_scorer.fit(repo_corpus_list)

        # BPE scorer: call-receiver soft-penalty scorer
        self._call_receiver: CallReceiverScorer | None = None
        if call_receiver_alpha > 0.0:
            self._call_receiver = CallReceiverScorer(
                repo_corpus_list,
                language=language_for_adapter(self._adapter),
                alpha=call_receiver_alpha,
                cap=call_receiver_cap,
                adapter=self._adapter,
                n_clusters=call_receiver_n_clusters,
                cluster_seed=call_receiver_cluster_seed,
                force_jaccard_routing=call_receiver_force_jaccard_routing,
                cluster_rare_threshold=call_receiver_cluster_rare_threshold,
                cluster_size_min=call_receiver_cluster_size_min,
                shape_primitives=call_receiver_shape_primitives,
            )
        self._call_receiver_root_bonus: float = call_receiver_root_bonus
        self._call_receiver_cluster_bonus: float = call_receiver_cluster_bonus

        # BPE tokenizer
        if _tokenizer is None:
            from transformers import AutoTokenizer

            from argot.ml.embeddings import _model_in_local_cache

            # Cache-first: skip the HF Hub metadata round-trip when the
            # tokenizer is already on disk.  Saves rate-limit budget across
            # the many calibration / bench / extraction invocations.
            local_only = _model_in_local_cache(_BPE_MODEL_NAME)
            _tokenizer = AutoTokenizer.from_pretrained(_BPE_MODEL_NAME, local_files_only=local_only)
        self._tokenizer = _tokenizer
        vocab: dict[str, int] = _tokenizer.get_vocab()
        self._id_to_token: dict[int, str] = {v: k for k, v in vocab.items()}

        # BPE generic baseline (broad open-source corpus reference)
        raw: dict[str, Any] = json.loads(bpe_generic_baseline_path.read_text(encoding="utf-8"))
        self._generic_baseline: dict[int, int] = {int(k): v for k, v in raw["token_counts"].items()}
        self._total_generic: int = raw["total_tokens"]

        # BPE repo corpus
        counts: Counter[int] = Counter()
        for path in repo_corpus_list:
            try:
                source = path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            ids: list[int] = _tokenizer.encode(source, add_special_tokens=False)
            counts.update(ids)
        self._repo_corpus: dict[int, int] = dict(counts)
        self._total_repo: int = sum(counts.values()) or 1  # avoid division by zero

        # Pre-computed evidence corpus loaded from calibration JSON; ``None``
        # in tests / inference paths that don't need evidence rendering. Per-
        # reason collectors (Step 3) consult this to populate ``common here:``
        # samples and rarity denominators.
        self._evidence_corpus: EvidenceCorpus | None = evidence_corpus

        if bpe_threshold is not None:
            # Use pre-computed threshold (e.g. loaded from scorer-config.json)
            self.bpe_threshold: float = bpe_threshold
            self.cal_scores: list[float] = []
            self.n_calibration: int = 0
        else:
            # Cluster-aware calibration path: when n_clusters>1 AND metadata is
            # supplied, fold the cluster_bonus contribution into calibration
            # scores so the per-corpus threshold absorbs the cluster-conditional
            # signal. alpha/root_bonus stay at 0.0 here so only cluster_bonus
            # shifts the calibration distribution.
            use_metadata_path = (
                calibration_hunks_with_metadata is not None
                and call_receiver_n_clusters > 1
                and self._call_receiver is not None
            )
            if use_metadata_path:
                assert calibration_hunks_with_metadata is not None  # for mypy
                assert self._call_receiver is not None  # for mypy
                meta_list: list[tuple[str, Path, str]] = list(calibration_hunks_with_metadata)
                if self._typicality_model is not None:
                    meta_list = [
                        t for t in meta_list if not self._typicality_model.is_atypical(t[0])[0]
                    ]
                cal_scores: list[float] = []
                for hunk, fp, src in meta_list:
                    raw_bpe: float = self._bpe_score(
                        _blank_prose_lines(hunk, self._adapter.prose_line_ranges(hunk))
                    )
                    contrib = self._call_receiver.weighted_contribution_for_file(
                        hunk,
                        file_path=fp,
                        file_source=src,
                        alpha=0.0,
                        root_bonus=0.0,
                        cluster_bonus=call_receiver_cluster_bonus,
                        cap=float(self._call_receiver.cap),
                    )
                    cal_scores.append(raw_bpe + contrib)
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

    @property
    def rare_branch_fire_count(self) -> int:
        """Total times the cluster-rare branch fired (0 when scoring disabled)."""
        return self._call_receiver.rare_branch_fire_count if self._call_receiver is not None else 0

    @property
    def rare_branch_hunks_fired(self) -> int:
        """Distinct hunks that fired the cluster-rare branch (0 when scoring disabled).
        Per-hunk fire rate is robust to "many fires per hunk vs few fires per hunk".
        """
        return self._call_receiver.rare_branch_hunks_fired if self._call_receiver is not None else 0

    @property
    def hunks_scored(self) -> int:
        """Total hunks scored by the call_receiver (denominator for fire-rate computations)."""
        return self._call_receiver.hunks_scored if self._call_receiver is not None else 0

    def _token_surprise(self, token_id: int) -> float:
        """Per-token log-likelihood ratio (generic baseline vs. repo corpus).

        The same formula :meth:`_bpe_score` aggregates over the whole hunk —
        factored out so the BPE evidence collector can rank individual
        tokens without duplicating the math.
        """
        return math.log(
            self._generic_baseline.get(token_id, 0) / self._total_generic + _EPSILON
        ) - math.log(self._repo_corpus.get(token_id, 0) / self._total_repo + _EPSILON)

    def _is_meaningful_token_id(self, token_id: int) -> bool:
        """Token-id form of :func:`_is_meaningful_token` for the collectors."""
        return _is_meaningful_token(self._id_to_token.get(token_id, ""))

    def _bpe_score(self, hunk_source: str) -> float:
        ids: list[int] = self._tokenizer.encode(hunk_source, add_special_tokens=False)
        filtered = [i for i in ids if _is_meaningful_token(self._id_to_token.get(i, ""))]
        if not filtered:
            filtered = ids
        if not filtered:
            return 0.0
        return max(self._token_surprise(i) for i in filtered)

    def score_hunk(
        self,
        hunk_content: str,
        *,
        file_source: str | None = None,
        hunk_start_line: int | None = None,
        hunk_end_line: int | None = None,
        file_path: Path | None = None,
    ) -> ScoredHunk:
        """Score a hunk through both stages.

        Args:
            hunk_content: The raw hunk diff / function body to score.
            file_source: Optional full source of the file containing the hunk.
                When provided, the import checker fires only on imports added in the hunk
                itself (not the file's header imports).  A pure string or comment
                edit in an import-heavy file will not trigger the import checker because
                the hunk has no import statements of its own.
                The BPE scorer always scores ``hunk_content`` only, regardless of
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
        provided, the BPE scorer blanks any prose lines (docstrings, comments) that
        fall within the hunk range before BPE scoring, mirroring the symmetric
        treatment applied to calibration hunks.

        file_path: Optional resolved path to the file containing the hunk. When
            provided, the BPE scorer uses weighted_contribution_for_file() which can
            apply an additive cluster_bonus for globally-attested callees absent
            from the file's cluster attested set. Has no effect when n_clusters=1
            (cluster-conditional scoring disabled).

        Returns a :class:`ScoredHunk` whose ``score`` / ``threshold`` belong
        to the winning reason — :data:`_IMPORT_THRESHOLD` for ``import``,
        ``self.bpe_threshold`` for ``bpe`` / ``call_receiver``. Raw per-stage
        scores are always available on ``ScoredHunk.stages`` for ML feature
        extraction and bench diagnostics; short-circuits return all-zero
        ``StageScores``.
        """
        # Typicality short-circuits (replaces the legacy auto-generated gate).
        if self._typicality_model is not None:
            is_atyp_hunk, _ = self._typicality_model.is_atypical(hunk_content)
            if is_atyp_hunk:
                return ScoredHunk(
                    score=0.0,
                    threshold=self.bpe_threshold,
                    flagged=False,
                    reason="atypical",
                    stages=StageScores(0.0, 0.0, 0.0),
                )
            if file_source is not None:
                is_atyp_file, _ = self._typicality_model.is_atypical_file(file_source)
                if is_atyp_file:
                    return ScoredHunk(
                        score=0.0,
                        threshold=self.bpe_threshold,
                        flagged=False,
                        reason="atypical_file",
                        stages=StageScores(0.0, 0.0, 0.0),
                    )
        elif file_source is not None and self._adapter.is_auto_generated(file_source):
            return ScoredHunk(
                score=0.0,
                threshold=self.bpe_threshold,
                flagged=False,
                reason="auto_generated",
                stages=StageScores(0.0, 0.0, 0.0),
            )

        if file_source is not None:
            # Import checker — hunk-only: only imports added in the hunk can introduce a
            # foreign module.  A pure string/comment edit in an import-heavy file
            # has no hunk imports and cannot trigger the import checker.
            hunk_imports = self._adapter.extract_imports(hunk_content)
            foreign: set[str] = {
                spec for spec in hunk_imports if self._import_scorer.is_foreign(spec)
            }
            import_score: float = float(len(foreign))
        else:
            # No file_source → score_hunk over the hunk text. We don't get
            # the foreign-set "for free" here, so recompute it for evidence.
            foreign = {
                spec
                for spec in self._adapter.extract_imports(hunk_content)
                if self._import_scorer.is_foreign(spec)
            }
            import_score = float(len(foreign))

        # BPE scorer: optionally blank prose lines before BPE scoring
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

        # Call-receiver contribution is always computed (when the scorer is
        # configured) so the diagnostic ``StageScores.call_receiver_contribution``
        # field is populated regardless of which stage wins. The value is also
        # used by Step 2's multi-reason ratio resolution.
        contribution: float = 0.0
        if self._call_receiver is not None:
            if file_path is not None:
                contribution = self._call_receiver.weighted_contribution_for_file(
                    hunk_content,
                    file_path,
                    alpha=self._call_receiver.alpha,
                    root_bonus=self._call_receiver_root_bonus,
                    cluster_bonus=self._call_receiver_cluster_bonus,
                    cap=float(self._call_receiver.cap),
                    file_source=file_source,
                )
            else:
                contribution = self._call_receiver.weighted_contribution(
                    hunk_content,
                    alpha=self._call_receiver.alpha,
                    root_bonus=self._call_receiver_root_bonus,
                    cap=float(self._call_receiver.cap),
                )

        stages = StageScores(
            import_score=import_score,
            bpe_score=bpe_score,
            call_receiver_contribution=contribution,
        )

        # ------------------------------------------------------------------
        # Multi-reason resolution (D3): every stage that crosses its own
        # threshold becomes a candidate; the candidate with the largest
        # ``score / threshold`` ratio wins, with a fixed precedence
        # ``call_receiver > import > bpe`` breaking ties for determinism.
        #
        # Why ratios and not raw ``max``: ``import_score`` is a count
        # against a constant 1.0 threshold; ``bpe_score`` is a log-likelihood
        # ratio against a per-repo calibrated threshold. Comparing the raw
        # values would let "1 foreign import" always beat "BPE 2.5× over
        # threshold". Normalising by each stage's own threshold puts every
        # signal on a common "how loud is this relative to typical" scale.
        #
        # ``bpe`` and ``call_receiver`` are deliberately disjoint here:
        # ``bpe`` fires when the raw score alone crosses threshold, and
        # ``call_receiver`` fires only when the call-receiver contribution
        # was the decisive factor (raw bpe ≤ threshold but adjusted > it).
        # That preserves today's interpretable split between "vocabulary
        # register tripped" and "unfamiliar callee tipped it".
        # ------------------------------------------------------------------
        adjusted_bpe = bpe_score + contribution
        cr_active = self._call_receiver is not None
        # Effective BPE-side score for the threshold comparison: use the
        # adjusted value when the call-receiver layer is enabled (matches
        # how the previous single-reason path behaved), so a BPE-only
        # config still gates strictly on raw bpe_score.
        bpe_side_score = adjusted_bpe if cr_active else bpe_score

        import_fired = import_score >= _IMPORT_THRESHOLD
        bpe_fired = bpe_score > self.bpe_threshold
        cr_fired = cr_active and not bpe_fired and bpe_side_score > self.bpe_threshold

        # Guard against a near-zero calibrated threshold producing infinite
        # ratios — clamp the denominator with a tiny epsilon. The order
        # between fired stages stays correct; a vanishing threshold just
        # means every stage looks equally "infinitely loud", which the
        # tiebreak precedence then resolves deterministically.
        denom = max(self.bpe_threshold, _EPSILON)

        # (reason, score, threshold, ratio) for each fired stage.
        candidates: list[tuple[Reason, float, float, float]] = []
        if import_fired:
            candidates.append(
                ("import", import_score, _IMPORT_THRESHOLD, import_score / _IMPORT_THRESHOLD)
            )
        if bpe_fired:
            candidates.append(("bpe", bpe_score, self.bpe_threshold, bpe_score / denom))
        if cr_fired:
            candidates.append(
                ("call_receiver", adjusted_bpe, self.bpe_threshold, adjusted_bpe / denom)
            )

        if candidates:
            # Highest ratio wins; tiebreak by a fixed precedence so two
            # stages reporting the same ratio always resolve the same way.
            tiebreak = {"call_receiver": 0, "import": 1, "bpe": 2}
            candidates.sort(key=lambda c: (-c[3], tiebreak[c[0]]))
            winner = candidates[0]
            evidence = self._collect_evidence(
                winner[0],
                hunk_content=hunk_content,
                bpe_input=bpe_input,
                file_path=file_path,
                file_source=file_source,
                foreign=foreign,
            )
            return ScoredHunk(
                score=winner[1],
                threshold=winner[2],
                flagged=True,
                reason=winner[0],
                stages=stages,
                evidence=evidence,
            )

        # Nothing fired. Headline score still describes a stage so the
        # dataclass stays rectangular; the renderer skips the hit via
        # ``flagged=False``.
        return ScoredHunk(
            score=bpe_side_score,
            threshold=self.bpe_threshold,
            flagged=False,
            reason="none",
            stages=stages,
        )

    def _collect_evidence(
        self,
        winning_reason: Reason,
        *,
        hunk_content: str,
        bpe_input: str,
        file_path: Path | None,
        file_source: str | None,
        foreign: set[str],
    ) -> Evidence | None:
        """Dispatch to the per-reason evidence collector after a hunk fires.

        Returns ``None`` when the scorer wasn't constructed with an
        :class:`EvidenceCorpus` — the renderer then skips the evidence
        block, matching the unflagged path. Lazy-imports the per-reason
        modules so callers that never need evidence (tests, ML feature
        extraction) don't pay for the import.
        """
        if self._evidence_corpus is None:
            return None

        # Imports kept local: see the ``# Local imports for evidence
        # collectors`` note at module top.
        from argot.scoring.evidence.bpe import collect_bpe_evidence
        from argot.scoring.evidence.call_receiver import collect_call_receiver_evidence
        from argot.scoring.evidence.imports import collect_import_evidence

        if winning_reason == "import":
            # Preserve the order the imports appear in the hunk for the
            # rendered ``↳`` line — sets don't preserve order, so re-derive.
            hunk_imports = list(self._adapter.extract_imports(hunk_content))
            ordered_foreign = [m for m in hunk_imports if m in foreign]
            return collect_import_evidence(
                foreign_specifiers=ordered_foreign,
                evidence_corpus=self._evidence_corpus,
            )
        if winning_reason == "call_receiver":
            if self._call_receiver is None:  # pragma: no cover — cr_fired guard
                return None
            cluster_id = self._call_receiver.cluster_id_for_hunk_file(file_path, file_source)
            return collect_call_receiver_evidence(
                unattested_callees=self._call_receiver.distinct_unattested(hunk_content),
                cluster_id=cluster_id,
                evidence_corpus=self._evidence_corpus,
            )
        if winning_reason == "bpe":
            return collect_bpe_evidence(
                hunk_source=bpe_input,
                tokenizer=self._tokenizer,
                score_fn=self._token_surprise,
                is_meaningful=self._is_meaningful_token_id,
                evidence_corpus=self._evidence_corpus,
            )
        return None
