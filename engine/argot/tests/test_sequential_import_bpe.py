from __future__ import annotations

from pathlib import Path

import pytest

from argot.scoring.adapters.python_adapter import PythonAdapter
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

_CATALOG = Path(__file__).parent.parent / "acceptance" / "catalog"
_FASTAPI_FIXTURES = _CATALOG / "fastapi" / "fixtures" / "default"
_BPE_MODEL_B = Path(__file__).parent.parent / "scoring" / "bpe" / "generic_tokens_bpe.json"

# Tiny inline model_a for unit tests (one real FastAPI file + two synthetics)
_CONTROL_FILES = sorted(_FASTAPI_FIXTURES.glob("control_*.py"))[:5]


def _make_scorer(
    model_a_files: list[Path] | None = None,
    calibration_hunks: list[str] | None = None,
    bpe_threshold: float | None = None,
) -> SequentialImportBpeScorer:
    files = model_a_files or _CONTROL_FILES
    return SequentialImportBpeScorer(
        model_a_files=files,
        bpe_model_b_path=_BPE_MODEL_B,
        calibration_hunks=calibration_hunks,
        bpe_threshold=bpe_threshold if bpe_threshold is not None else None,
    )


def test_stage1_flags_foreign_import() -> None:
    """Stage 1 fires when hunk introduces a module absent from model_A."""
    scorer = _make_scorer(bpe_threshold=99.0)  # disable stage 2
    hunk = "import flask\nfrom flask import Flask, request\napp = Flask(__name__)\n"
    result = scorer.score_hunk(hunk)
    assert result["import_score"] >= 1.0, "flask is not in FastAPI corpus — should flag"
    assert result["flagged"] is True
    assert result["reason"] == "import"


def test_stage1_clean_on_known_import() -> None:
    """Stage 1 does not fire for modules already seen in model_A."""
    scorer = _make_scorer(bpe_threshold=99.0)
    # fastapi is a known import in the FastAPI corpus
    hunk = "from fastapi import APIRouter\nrouter = APIRouter()\n"
    result = scorer.score_hunk(hunk)
    assert result["import_score"] == 0.0


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
    assert result["import_score"] == 0.0, "Hunk-scope: hunk has no imports, should not fire"


def test_stage2_bpe_score_is_float() -> None:
    """Stage 2 always returns a float bpe_score."""
    scorer = _make_scorer(bpe_threshold=0.0)
    result = scorer.score_hunk("x = 1\n")
    assert isinstance(result["bpe_score"], float)


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
    assert result_with_source["bpe_score"] <= result_without_source["bpe_score"]


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
    with pytest.raises(ValueError, match="Either calibration_hunks or bpe_threshold"):
        SequentialImportBpeScorer(
            model_a_files=_CONTROL_FILES,
            bpe_model_b_path=_BPE_MODEL_B,
        )


def test_scorer_filters_data_dominant_model_a_files(tmp_path: Path) -> None:
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
    # Data-dominant file — should be filtered out of model A.
    data_file = tmp_path / "data.py"
    data_file.write_text("DATA = {\n" + "\n".join(f'  "k{i}": "v{i}",' for i in range(120)) + "\n}")

    bpe_model_b = tmp_path / "bpe.json"
    bpe_model_b.write_text('{"token_counts": {}, "total_tokens": 1}')

    scorer = SequentialImportBpeScorer(
        model_a_files=[code_file, data_file],
        bpe_model_b_path=bpe_model_b,
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
    bpe_model_b = tmp_path / "bpe.json"
    bpe_model_b.write_text('{"token_counts": {}, "total_tokens": 1}')

    normal_hunk = (
        "def other(x, registry):\n"
        "    items = registry.lookup(x)\n"
        "    if items:\n"
        "        return items[0]\n"
        "    return None\n"
    )
    data_hunk = "DATA = {\n" + "\n".join(f'  "k{i}": "v{i}",' for i in range(80)) + "\n}"

    scorer = SequentialImportBpeScorer(
        model_a_files=[code_file],
        bpe_model_b_path=bpe_model_b,
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
    bpe_model_b = tmp_path / "bpe.json"
    bpe_model_b.write_text('{"token_counts": {}, "total_tokens": 1}')
    cal = "def g(x):\n    if x:\n        return 1\n    return 2\n"

    return SequentialImportBpeScorer(
        model_a_files=[code_file],
        bpe_model_b_path=bpe_model_b,
        calibration_hunks=[cal],
        adapter=PythonAdapter(),
    )


def test_score_hunk_short_circuits_atypical_hunk(tmp_path: Path) -> None:
    scorer = _make_scorer_for_score_hunk_tests(tmp_path)
    data_hunk = "\n".join(["EMOJI = {", *(f'    "e{i}": "U+{i:05X}",' for i in range(60)), "}"])
    result = scorer.score_hunk(data_hunk)
    assert result["flagged"] is False
    assert result["reason"] == "atypical"


def test_score_hunk_short_circuits_atypical_file(tmp_path: Path) -> None:
    scorer = _make_scorer_for_score_hunk_tests(tmp_path)
    # Hunk itself is short, but its enclosing file_source is data-dominant.
    file_source = "DATA = {\n" + "\n".join(f'  "k{i}": "v{i}",' for i in range(120)) + "\n}"
    # Pull a small slice as the hunk content.
    hunk_content = '  "k1": "v1",\n  "k2": "v2",'
    result = scorer.score_hunk(
        hunk_content, file_source=file_source, hunk_start_line=2, hunk_end_line=3
    )
    assert result["flagged"] is False
    assert result["reason"] == "atypical_file"


def test_score_hunk_scores_normal_code_normally(tmp_path: Path) -> None:
    scorer = _make_scorer_for_score_hunk_tests(tmp_path)
    normal_hunk = (
        "def other(x, registry):\n    if x:\n        return registry[x]\n    return None\n"
    )
    result = scorer.score_hunk(normal_hunk)
    # Not short-circuited — reason is one of the normal values.
    assert result["reason"] in ("none", "import", "bpe")


# ---------------------------------------------------------------------------
# Stage 1.5 call-receiver integration tests
# ---------------------------------------------------------------------------


def test_call_receiver_disabled_when_alpha_zero(tmp_path: Path) -> None:
    """alpha=0.0 disables Stage 1.5 — no call_receiver scorer built."""
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    code_file = tmp_path / "code.py"
    code_file.write_text("def fn(x):\n    logger.info(x)\n    return x\n")
    bpe_model_b = tmp_path / "bpe.json"
    bpe_model_b.write_text('{"token_counts": {}, "total_tokens": 1}')

    scorer = SequentialImportBpeScorer(
        model_a_files=[code_file],
        bpe_model_b_path=bpe_model_b,
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
    bpe_model_b = tmp_path / "bpe.json"
    bpe_model_b.write_text('{"token_counts": {}, "total_tokens": 1}')

    scorer = SequentialImportBpeScorer(
        model_a_files=[code_file],
        bpe_model_b_path=bpe_model_b,
        bpe_threshold=0.5,
        call_receiver_alpha=1.0,
        call_receiver_cap=5,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
    )
    assert isinstance(scorer._call_receiver, CallReceiverScorer)
    assert scorer._call_receiver.alpha == 1.0
    assert scorer._call_receiver.cap == 5


def test_import_reason_takes_precedence_over_call_receiver(tmp_path: Path) -> None:
    """Stage 1 import fires → reason='import' regardless of call-receiver."""
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    code_file = tmp_path / "code.py"
    code_file.write_text(
        "import logging\nlogger = logging.getLogger()\n"
        "def fn(x):\n    logger.info(x)\n    return x\n"
    )
    bpe_model_b = tmp_path / "bpe.json"
    bpe_model_b.write_text('{"token_counts": {}, "total_tokens": 1}')

    scorer = SequentialImportBpeScorer(
        model_a_files=[code_file],
        bpe_model_b_path=bpe_model_b,
        bpe_threshold=0.0,
        call_receiver_alpha=1.0,
        call_receiver_cap=5,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
    )

    result = scorer.score_hunk("import flask\nflask.Flask(__name__)")
    assert result["reason"] == "import"
    assert result["flagged"] is True


def test_no_flag_when_all_callees_attested(tmp_path: Path) -> None:
    """No unattested callees + low raw BPE → reason='none'."""
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    code_file = tmp_path / "code.py"
    code_file.write_text(
        "import logging\nlogger = logging.getLogger()\n"
        "def fn(x):\n    logger.info(x)\n    logger.debug(x)\n    return x\n"
    )
    bpe_model_b = tmp_path / "bpe.json"
    bpe_model_b.write_text('{"token_counts": {}, "total_tokens": 1}')

    scorer = SequentialImportBpeScorer(
        model_a_files=[code_file],
        bpe_model_b_path=bpe_model_b,
        bpe_threshold=999.0,
        call_receiver_alpha=1.0,
        call_receiver_cap=5,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
    )

    # All callees (logger.info, logger.debug) are attested
    result = scorer.score_hunk("logger.info('hello')\nlogger.debug('world')")
    assert result["flagged"] is False
    assert result["reason"] == "none"


def test_call_receiver_reason_when_penalty_tips_threshold(tmp_path: Path) -> None:
    """Unattested callees + soft penalty tips threshold → reason='call_receiver'."""
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    code_file = tmp_path / "code.py"
    code_file.write_text(
        "import logging\nlogger = logging.getLogger()\n"
        "def fn(x):\n    logger.info(x)\n    return x\n"
    )
    bpe_model_b = tmp_path / "bpe.json"
    # All tokens in model_b to keep raw BPE low (common tokens → low log-ratio)
    bpe_model_b.write_text('{"token_counts": {}, "total_tokens": 1}')

    # threshold=0.5: raw BPE on normal code is typically << 0.5 with empty model_b
    # But with empty model_b, log(0/1 + eps) - log(count/total + eps) → values vary.
    # Use threshold=999 so raw BPE never trips; alpha=1.0, cap=5; 3 unattested → adjusted = bpe+3
    # Actually: bpe+3 > 0.5 will definitely fire call_receiver if bpe < 0.5
    # Use a very permissive threshold on raw BPE:
    scorer = SequentialImportBpeScorer(
        model_a_files=[code_file],
        bpe_model_b_path=bpe_model_b,
        bpe_threshold=0.5,
        call_receiver_alpha=1.0,
        call_receiver_cap=5,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
    )

    # 3 unattested callees: Math.random, crypto.randomBytes, axios.get
    result = scorer.score_hunk("Math.random()\ncrypto.randomBytes(16)\naxios.get('/foo')")
    assert result["flagged"] is True
    if result["bpe_score"] <= 0.5:
        assert result["reason"] == "call_receiver", (
            f"Expected call_receiver, got {result['reason']!r} "
            f"(bpe_score={result['bpe_score']}, threshold=0.5)"
        )
