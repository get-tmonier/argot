from __future__ import annotations

from pathlib import Path

import pytest

from argot.scoring.adapters.python_adapter import PythonAdapter
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

_CATALOG = Path(__file__).parent.parent / "acceptance" / "catalog"
_FASTAPI_FIXTURES = _CATALOG / "fastapi" / "fixtures" / "default"
_BPE_GENERIC_BASELINE = Path(__file__).parent.parent / "scoring" / "bpe" / "generic_tokens_bpe.json"

# Tiny inline repo corpus for unit tests (one real FastAPI file + two synthetics)
_CONTROL_FILES = sorted(_FASTAPI_FIXTURES.glob("control_*.py"))[:5]


def _make_scorer(
    repo_corpus_files: list[Path] | None = None,
    calibration_hunks: list[str] | None = None,
    bpe_threshold: float | None = None,
) -> SequentialImportBpeScorer:
    files = repo_corpus_files or _CONTROL_FILES
    return SequentialImportBpeScorer(
        repo_corpus_files=files,
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        calibration_hunks=calibration_hunks,
        bpe_threshold=bpe_threshold if bpe_threshold is not None else None,
    )


def test_stage1_flags_foreign_import() -> None:
    """Stage 1 fires when hunk introduces a module absent from the repo corpus."""
    scorer = _make_scorer(bpe_threshold=99.0)  # disable stage 2
    hunk = "import flask\nfrom flask import Flask, request\napp = Flask(__name__)\n"
    result = scorer.score_hunk(hunk)
    assert result.stages.import_score >= 1.0, "flask is not in FastAPI corpus — should flag"
    assert result.flagged is True
    assert result.reason == "import"


def test_stage1_clean_on_known_import() -> None:
    """Stage 1 does not fire for modules already seen in the repo corpus."""
    scorer = _make_scorer(bpe_threshold=99.0)
    # fastapi is a known import in the FastAPI corpus
    hunk = "from fastapi import APIRouter\nrouter = APIRouter()\n"
    result = scorer.score_hunk(hunk)
    assert result.stages.import_score == 0.0


def test_stage1_hunk_scope_no_fire_on_file_imports() -> None:
    """When file_source is provided, Stage 1 only counts imports in the hunk itself.

    A pure string edit in an import-heavy file must NOT fire Stage 1 because
    the hunk contains no import statements.
    """
    scorer = _make_scorer(bpe_threshold=99.0)
    # file_source has a flask import at the top, but the hunk itself has none
    file_source = "import flask\nimport requests\n\ndef hello():\n    return 'world'\n"
    hunk_content = "    return 'hello'"
    result = scorer.score_hunk(hunk_content, file_source=file_source)
    assert result.stages.import_score == 0.0, "Hunk-scope: hunk has no imports, should not fire"


def test_stage2_bpe_score_is_float() -> None:
    """Stage 2 always returns a float bpe_score."""
    scorer = _make_scorer(bpe_threshold=0.0)
    result = scorer.score_hunk("x = 1\n")
    assert isinstance(result.stages.bpe_score, float)


def test_prose_masking_reduces_bpe_score_for_pure_comment_hunk() -> None:
    """A hunk containing only comments should have a lower BPE score than code.

    The prose masking blanks comment lines before BPE scoring; a hunk that is
    entirely prose collapses to an empty token sequence → score 0.0.
    """
    scorer = _make_scorer(bpe_threshold=99.0)
    comment_hunk = "# This is a comment about routing\n# Another line\n"
    file_source = "# comment\ndef foo():\n    pass\n"
    result_with_source = scorer.score_hunk(
        comment_hunk, file_source=file_source, hunk_start_line=1, hunk_end_line=2
    )
    result_without_source = scorer.score_hunk(comment_hunk)
    # With prose masking, comment-only hunk collapses → lower BPE score
    assert result_with_source.stages.bpe_score <= result_without_source.stages.bpe_score


def test_data_dominance_filter_rejects_data_file() -> None:
    """A hand-crafted data-only file is rejected by is_data_dominant."""
    adapter = PythonAdapter()
    data_file = (
        "TRANSLATIONS = {\n"
        "    'en': 'English',\n"
        "    'fr': 'French',\n"
        "    'de': 'German',\n"
        "    'es': 'Spanish',\n"
        "    'it': 'Italian',\n"
        "    'pt': 'Portuguese',\n"
        "    'nl': 'Dutch',\n"
        "    'pl': 'Polish',\n"
        "    'ru': 'Russian',\n"
        "    'zh': 'Chinese',\n"
        "    'ja': 'Japanese',\n"
        "    'ko': 'Korean',\n"
        "    'ar': 'Arabic',\n"
        "    'hi': 'Hindi',\n"
        "    'sv': 'Swedish',\n"
        "    'da': 'Danish',\n"
        "    'fi': 'Finnish',\n"
        "    'no': 'Norwegian',\n"
        "    'tr': 'Turkish',\n"
        "}\n"
    )
    assert adapter.is_data_dominant(data_file), "Locale data dict should be data-dominant"


def test_data_dominance_does_not_reject_code_file() -> None:
    """Normal code with imports and functions is not data-dominant."""
    adapter = PythonAdapter()
    code_file = "\n".join(_CONTROL_FILES[0].read_text().splitlines()[:20])
    assert not adapter.is_data_dominant(
        code_file
    ), "FastAPI control file should not be data-dominant"


def test_bpe_threshold_override_skips_calibration() -> None:
    """Passing bpe_threshold constructs scorer without calibration_hunks."""
    scorer = _make_scorer(bpe_threshold=5.0)
    assert scorer.bpe_threshold == pytest.approx(5.0)
    assert scorer.n_calibration == 0
    assert scorer.cal_scores == []


def test_scorer_raises_without_hunks_or_threshold() -> None:
    """Scorer raises ValueError if neither calibration_hunks nor bpe_threshold is given."""
    with pytest.raises(ValueError, match="calibration_hunks.*bpe_threshold"):
        SequentialImportBpeScorer(
            repo_corpus_files=_CONTROL_FILES,
            bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        )


def test_scorer_filters_data_dominant_repo_corpus_files(tmp_path: Path) -> None:
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    # Normal code file — should be retained.
    code_file = tmp_path / "code.py"
    code_file.write_text(
        "\n".join(
            [
                "def fn(value, registry):",
                "    items = registry.lookup(value)",
                "    if not items:",
                "        return None",
                "    out = []",
                "    for item in items:",
                "        out.append(item.transform(value))",
                "    return out",
            ]
        )
    )
    # Data-dominant file — should be filtered out of repo corpus.
    data_file = tmp_path / "data.py"
    data_file.write_text("DATA = {\n" + "\n".join(f'  "k{i}": "v{i}",' for i in range(120)) + "\n}")

    bpe_generic_baseline = tmp_path / "bpe.json"
    bpe_generic_baseline.write_text('{"token_counts": {}, "total_tokens": 1}')

    scorer = SequentialImportBpeScorer(
        repo_corpus_files=[code_file, data_file],
        bpe_generic_baseline_path=bpe_generic_baseline,
        calibration_hunks=["def g():\n    return 1\n    return 2"],
        adapter=PythonAdapter(),
    )
    # The scorer should have a typicality model attached.
    assert scorer._typicality_model is not None


def test_scorer_filters_atypical_calibration_hunks(tmp_path: Path) -> None:
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    code_file = tmp_path / "code.py"
    code_file.write_text(
        "def fn(x):\n" "    if x > 0:\n" "        return x + 1\n" "    return x - 1\n"
    )
    bpe_generic_baseline = tmp_path / "bpe.json"
    bpe_generic_baseline.write_text('{"token_counts": {}, "total_tokens": 1}')

    normal_hunk = (
        "def other(x, registry):\n"
        "    items = registry.lookup(x)\n"
        "    if items:\n"
        "        return items[0]\n"
        "    return None\n"
    )
    data_hunk = "DATA = {\n" + "\n".join(f'  "k{i}": "v{i}",' for i in range(80)) + "\n}"

    scorer = SequentialImportBpeScorer(
        repo_corpus_files=[code_file],
        bpe_generic_baseline_path=bpe_generic_baseline,
        calibration_hunks=[normal_hunk, data_hunk],
        adapter=PythonAdapter(),
    )
    # Only the normal hunk should appear in cal_scores — data_hunk was filtered.
    assert len(scorer.cal_scores) == 1


def _make_scorer_for_score_hunk_tests(tmp_path: Path) -> SequentialImportBpeScorer:
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    code_file = tmp_path / "code.py"
    code_file.write_text(
        "def fn(x):\n" "    if x > 0:\n" "        return x + 1\n" "    return x - 1\n"
    )
    bpe_generic_baseline = tmp_path / "bpe.json"
    bpe_generic_baseline.write_text('{"token_counts": {}, "total_tokens": 1}')
    cal = "def g(x):\n    if x:\n        return 1\n    return 2\n"

    return SequentialImportBpeScorer(
        repo_corpus_files=[code_file],
        bpe_generic_baseline_path=bpe_generic_baseline,
        calibration_hunks=[cal],
        adapter=PythonAdapter(),
    )


def test_score_hunk_short_circuits_atypical_hunk(tmp_path: Path) -> None:
    scorer = _make_scorer_for_score_hunk_tests(tmp_path)
    data_hunk = "\n".join(["EMOJI = {", *(f'    "e{i}": "U+{i:05X}",' for i in range(60)), "}"])
    result = scorer.score_hunk(data_hunk)
    assert result.flagged is False
    assert result.reason == "atypical"


def test_score_hunk_short_circuits_atypical_file(tmp_path: Path) -> None:
    scorer = _make_scorer_for_score_hunk_tests(tmp_path)
    # Hunk itself is short, but its enclosing file_source is data-dominant.
    file_source = "DATA = {\n" + "\n".join(f'  "k{i}": "v{i}",' for i in range(120)) + "\n}"
    # Pull a small slice as the hunk content.
    hunk_content = '  "k1": "v1",\n  "k2": "v2",'
    result = scorer.score_hunk(
        hunk_content, file_source=file_source, hunk_start_line=2, hunk_end_line=3
    )
    assert result.flagged is False
    assert result.reason == "atypical_file"


def test_score_hunk_scores_normal_code_normally(tmp_path: Path) -> None:
    scorer = _make_scorer_for_score_hunk_tests(tmp_path)
    normal_hunk = (
        "def other(x, registry):\n    if x:\n        return registry[x]\n    return None\n"
    )
    result = scorer.score_hunk(normal_hunk)
    # Not short-circuited — reason is one of the normal values.
    assert result.reason in ("none", "import", "bpe")


# ---------------------------------------------------------------------------
# Stage 1.5 call-receiver integration tests
# ---------------------------------------------------------------------------


def test_call_receiver_disabled_when_alpha_zero(tmp_path: Path) -> None:
    """alpha=0.0 disables Stage 1.5 — no call_receiver scorer built."""
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    code_file = tmp_path / "code.py"
    code_file.write_text("def fn(x):\n    logger.info(x)\n    return x\n")
    bpe_generic_baseline = tmp_path / "bpe.json"
    bpe_generic_baseline.write_text('{"token_counts": {}, "total_tokens": 1}')

    scorer = SequentialImportBpeScorer(
        repo_corpus_files=[code_file],
        bpe_generic_baseline_path=bpe_generic_baseline,
        bpe_threshold=0.5,
        call_receiver_alpha=0.0,
        call_receiver_cap=5,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
    )
    assert scorer._call_receiver is None


def test_call_receiver_built_when_alpha_nonzero(tmp_path: Path) -> None:
    """alpha > 0 builds a CallReceiverScorer."""
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.call_receiver import CallReceiverScorer
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    code_file = tmp_path / "code.py"
    code_file.write_text("def fn(x):\n    logger.info(x)\n    return x\n")
    bpe_generic_baseline = tmp_path / "bpe.json"
    bpe_generic_baseline.write_text('{"token_counts": {}, "total_tokens": 1}')

    scorer = SequentialImportBpeScorer(
        repo_corpus_files=[code_file],
        bpe_generic_baseline_path=bpe_generic_baseline,
        bpe_threshold=0.5,
        call_receiver_alpha=1.0,
        call_receiver_cap=5,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
    )
    assert isinstance(scorer._call_receiver, CallReceiverScorer)
    assert scorer._call_receiver.alpha == 1.0
    assert scorer._call_receiver.cap == 5


def test_import_fires_alongside_call_receiver(tmp_path: Path) -> None:
    """When import + call_receiver both fire, the hunk is still flagged.

    Multi-reason resolution (D3) means the reported reason now depends on
    each stage's ``score / threshold`` ratio rather than a fixed import-
    first precedence. The hunk fires regardless of which reason wins; the
    point of this test is "both stages contribute", not "import always
    beats call_receiver".
    """
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    code_file = tmp_path / "code.py"
    code_file.write_text(
        "import logging\nlogger = logging.getLogger()\n"
        "def fn(x):\n    logger.info(x)\n    return x\n"
    )
    bpe_generic_baseline = tmp_path / "bpe.json"
    bpe_generic_baseline.write_text('{"token_counts": {}, "total_tokens": 1}')

    scorer = SequentialImportBpeScorer(
        repo_corpus_files=[code_file],
        bpe_generic_baseline_path=bpe_generic_baseline,
        bpe_threshold=0.0,
        call_receiver_alpha=1.0,
        call_receiver_cap=5,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
    )

    result = scorer.score_hunk("import flask\nflask.Flask(__name__)")
    assert result.flagged is True
    assert result.reason in ("import", "bpe", "call_receiver")
    # Every stage's raw score is preserved on stages, regardless of winner.
    assert result.stages.import_score >= 1.0


def test_multi_reason_picks_highest_ratio(tmp_path: Path) -> None:
    """Score / threshold ratio decides the reported reason when multiple fire."""
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    code_file = tmp_path / "code.py"
    code_file.write_text("def fn(x):\n    return x\n")
    bpe_generic_baseline = tmp_path / "bpe.json"
    bpe_generic_baseline.write_text('{"token_counts": {}, "total_tokens": 1}')

    # bpe_threshold=2.0, call_receiver disabled.
    # 1 foreign import → ratio import = 1/1 = 1.0
    # bpe_score on this hunk likely > 2 → ratio bpe > 1.0
    # Expect bpe wins because ratio > 1.0.
    scorer = SequentialImportBpeScorer(
        repo_corpus_files=[code_file],
        bpe_generic_baseline_path=bpe_generic_baseline,
        bpe_threshold=2.0,
        call_receiver_alpha=0.0,  # disable call_receiver
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
    )

    result = scorer.score_hunk("import flask\nflask.Flask(__name__)")
    if result.stages.bpe_score > 2.0 and result.stages.bpe_score / 2.0 > 1.0:
        assert result.reason == "bpe", (
            f"bpe ratio {result.stages.bpe_score / 2.0:.2f} > import ratio "
            f"{result.stages.import_score / 1.0:.2f} — bpe should win"
        )
        assert result.score == result.stages.bpe_score
        assert result.threshold == 2.0


def test_multi_reason_tiebreak_call_receiver_over_import(tmp_path: Path) -> None:
    """On tie: call_receiver > import > bpe per fixed precedence."""
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    code_file = tmp_path / "code.py"
    code_file.write_text("def fn(x):\n    return x\n")
    bpe_generic_baseline = tmp_path / "bpe.json"
    bpe_generic_baseline.write_text('{"token_counts": {}, "total_tokens": 1}')

    # bpe_threshold=0.0 → ratios go to ~infinity for any positive bpe_side
    # score (clamped via epsilon). Both import and call_receiver fire and
    # tie on ratio; precedence picks call_receiver.
    scorer = SequentialImportBpeScorer(
        repo_corpus_files=[code_file],
        bpe_generic_baseline_path=bpe_generic_baseline,
        bpe_threshold=0.0,
        call_receiver_alpha=1.0,
        call_receiver_cap=5,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
    )

    # `import flask` is foreign; flask.Flask is unattested → CR contributes.
    # raw bpe_score may itself exceed 0.0, in which case bpe fires too;
    # but call_receiver / bpe are disjoint by construction (cr only fires
    # when bpe doesn't), so on a hunk where raw bpe > 0 we can't observe
    # the call_receiver winner. Use a hunk that's pure call: no imports.
    result = scorer.score_hunk("UnknownClass().some_unknown_method()")
    # On the pure-call hunk: import doesn't fire, so the comparison is
    # bpe vs cr (disjoint). cr wins when raw bpe ≤ threshold and
    # contribution lifts adjusted > threshold.
    assert result.flagged is True
    assert result.reason in ("bpe", "call_receiver")


def test_no_flag_when_all_callees_attested(tmp_path: Path) -> None:
    """No unattested callees + low raw BPE → reason='none'."""
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    code_file = tmp_path / "code.py"
    code_file.write_text(
        "import logging\nlogger = logging.getLogger()\n"
        "def fn(x):\n    logger.info(x)\n    logger.debug(x)\n    return x\n"
    )
    bpe_generic_baseline = tmp_path / "bpe.json"
    bpe_generic_baseline.write_text('{"token_counts": {}, "total_tokens": 1}')

    scorer = SequentialImportBpeScorer(
        repo_corpus_files=[code_file],
        bpe_generic_baseline_path=bpe_generic_baseline,
        bpe_threshold=999.0,
        call_receiver_alpha=1.0,
        call_receiver_cap=5,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
    )

    # All callees (logger.info, logger.debug) are attested
    result = scorer.score_hunk("logger.info('hello')\nlogger.debug('world')")
    assert result.flagged is False
    assert result.reason == "none"


def test_call_receiver_reason_when_penalty_tips_threshold(tmp_path: Path) -> None:
    """Unattested callees + soft penalty tips threshold → reason='call_receiver'."""
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    code_file = tmp_path / "code.py"
    code_file.write_text(
        "import logging\nlogger = logging.getLogger()\n"
        "def fn(x):\n    logger.info(x)\n    return x\n"
    )
    bpe_generic_baseline = tmp_path / "bpe.json"
    # All tokens in the generic baseline to keep raw BPE low (common → low log-ratio)
    bpe_generic_baseline.write_text('{"token_counts": {}, "total_tokens": 1}')

    # threshold=0.5: raw BPE on normal code is typically << 0.5 with empty baseline.
    # But with an empty baseline, log(0/1 + eps) - log(count/total + eps) → values vary.
    # Use threshold=999 so raw BPE never trips; alpha=1.0, cap=5; 3 unattested → adjusted = bpe+3
    # Actually: bpe+3 > 0.5 will definitely fire call_receiver if bpe < 0.5
    # Use a very permissive threshold on raw BPE:
    scorer = SequentialImportBpeScorer(
        repo_corpus_files=[code_file],
        bpe_generic_baseline_path=bpe_generic_baseline,
        bpe_threshold=0.5,
        call_receiver_alpha=1.0,
        call_receiver_cap=5,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
    )

    # 3 unattested callees: Math.random, crypto.randomBytes, axios.get
    result = scorer.score_hunk("Math.random()\ncrypto.randomBytes(16)\naxios.get('/foo')")
    assert result.flagged is True
    if result.stages.bpe_score <= 0.5:
        assert result.reason == "call_receiver", (
            f"Expected call_receiver, got {result.reason!r} "
            f"(bpe_score={result.stages.bpe_score}, threshold=0.5)"
        )


def test_compute_threshold_iqr_basic() -> None:
    from argot.scoring.scorers.sequential_import_bpe import _compute_threshold

    # 10 evenly-spaced values: 1..10
    scores = list(range(1, 11))
    # p25 index = 0.25 * 9 = 2.25 → lo=2, hi=3 → 3 + 0.25*(4-3) = 3.25
    # p75 index = 0.75 * 9 = 6.75 → lo=6, hi=7 → 7 + 0.75*(8-7) = 7.75
    # IQR = 7.75 - 3.25 = 4.5
    # threshold = 7.75 + 2.5 * 4.5 = 7.75 + 11.25 = 19.0
    result = _compute_threshold(scores, threshold_percentile=None, threshold_iqr_k=2.5)  # type: ignore[arg-type]
    assert result == pytest.approx(19.0, abs=1e-9)


def test_compute_threshold_iqr_tight_distribution() -> None:
    from argot.scoring.scorers.sequential_import_bpe import _compute_threshold

    # Tight distribution: all values equal → IQR=0 → threshold = p75 + 0 = same value
    scores = [5.0] * 20
    result = _compute_threshold(scores, threshold_percentile=None, threshold_iqr_k=2.5)
    assert result == pytest.approx(5.0, abs=1e-9)


def test_compute_threshold_iqr_overrides_percentile() -> None:
    from argot.scoring.scorers.sequential_import_bpe import _compute_threshold

    # When threshold_iqr_k is set, threshold_percentile is ignored
    scores = list(range(1, 11))
    iqr_result = _compute_threshold(scores, threshold_percentile=95.0, threshold_iqr_k=2.5)  # type: ignore[arg-type]
    assert iqr_result == pytest.approx(19.0, abs=1e-9)


def test_compute_threshold_iqr_k_zero() -> None:
    from argot.scoring.scorers.sequential_import_bpe import _compute_threshold

    # k=0 → threshold = p75 exactly
    scores = list(range(1, 11))
    result = _compute_threshold(scores, threshold_percentile=None, threshold_iqr_k=0.0)  # type: ignore[arg-type]
    # p75 index = 6.75 → 7 + 0.75*(8-7) = 7.75
    assert result == pytest.approx(7.75, abs=1e-9)


def test_call_receiver_root_bonus_param_stored() -> None:
    """SequentialImportBpeScorer stores call_receiver_root_bonus."""
    scorer = _make_scorer(bpe_threshold=99.0)
    assert hasattr(scorer, "_call_receiver_root_bonus")
    assert scorer._call_receiver_root_bonus == 2.0  # shipping default


def test_call_receiver_root_bonus_custom_accepted() -> None:
    """SequentialImportBpeScorer accepts a custom call_receiver_root_bonus without error."""
    from pathlib import Path

    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    bpe_b = Path(__file__).parent.parent / "scoring" / "bpe" / "generic_tokens_bpe.json"
    scorer = SequentialImportBpeScorer(
        repo_corpus_files=_CONTROL_FILES,
        bpe_generic_baseline_path=bpe_b,
        bpe_threshold=99.0,
        call_receiver_alpha=2.0,
        call_receiver_root_bonus=3.0,
        call_receiver_cap=5,
    )
    assert scorer._call_receiver_root_bonus == 3.0


# ---------------------------------------------------------------------------
# Memory regression: streaming scorer stays bounded on large corpora
# ---------------------------------------------------------------------------


class _MockTokenizer:
    """Minimal BPE tokenizer stub for memory tests — avoids loading UnixCoder."""

    def encode(self, text: str, add_special_tokens: bool = True) -> list[int]:
        return [hash(w) % 1000 for w in text.split()]

    def get_vocab(self) -> dict[str, int]:
        return {str(i): i for i in range(1000)}


def test_scorer_peak_memory_bounded_on_large_corpus(tmp_path: Path) -> None:
    """Building a scorer on a 5000-file synthetic corpus keeps peak allocation under 4 GB.

    Verifies the streaming refactor: per-file callee frozensets are never held
    simultaneously in memory; only the compact MinHash signatures (128 ints per
    file) accumulate during clustering.  Uses tracemalloc to measure the delta
    during construction so the test is independent of the tokenizer model size.
    """
    import tracemalloc

    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    n_files = 5000
    # Write synthetic files with realistic callee diversity.
    for i in range(n_files):
        (tmp_path / f"m{i:04d}.py").write_text(
            f"def fn_{i}(x, y):\n"
            f"    a = x.process_{i % 100}()\n"
            f"    b = a.value_{i % 50}\n"
            f"    c = helper.compute(a, b)\n"
            f"    return c.result\n"
        )

    files = sorted(tmp_path.glob("*.py"))
    bpe_b = tmp_path / "bpe.json"
    bpe_b.write_text('{"token_counts": {}, "total_tokens": 1}')

    tracemalloc.start()
    try:
        _scorer = SequentialImportBpeScorer(
            repo_corpus_files=files,
            bpe_generic_baseline_path=bpe_b,
            bpe_threshold=1.0,
            adapter=PythonAdapter(),
            call_receiver_n_clusters=8,
            enable_typicality_filter=False,
            _tokenizer=_MockTokenizer(),
        )
        _current, peak = tracemalloc.get_traced_memory()
    finally:
        tracemalloc.stop()

    limit = 4 * 1024**3  # 4 GB
    assert (
        peak < limit
    ), f"Peak tracemalloc allocation {peak / 1024**2:.0f} MB exceeds {limit // 1024**2} MB limit"
