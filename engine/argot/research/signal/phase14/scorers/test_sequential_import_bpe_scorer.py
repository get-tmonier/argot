# engine/argot/research/signal/phase14/scorers/test_sequential_import_bpe_scorer.py
"""Tests for SequentialImportBpeScorer."""

from __future__ import annotations

import json
import math
from pathlib import Path

from argot.research.signal.phase14.scorers.sequential_import_bpe_scorer import (
    SequentialImportBpeScorer,
    _is_meaningful_token,
    extract_imports,
)

_EPSILON = 1e-7

# ---------------------------------------------------------------------------
# Fake tokenizer for deterministic BPE scores without downloading a model
# ---------------------------------------------------------------------------


class _FakeTok:
    """Deterministic tokenizer that maps exact source strings to known token IDs."""

    def __init__(self, source_to_ids: dict[str, list[int]]) -> None:
        self._map = source_to_ids
        # Vocab: long alphanumeric names so they pass the meaningful-token filter
        self._vocab: dict[str, int] = {f"token_{i:03d}": i for i in range(1, 10)}

    def encode(self, source: str, *, add_special_tokens: bool = False) -> list[int]:
        return list(self._map.get(source, []))

    def get_vocab(self) -> dict[str, int]:
        return dict(self._vocab)


def _make_model_b(tmp_path: Path, counts: dict[int, int]) -> Path:
    total = sum(counts.values())
    payload = {"token_counts": {str(k): v for k, v in counts.items()}, "total_tokens": total}
    p = tmp_path / "model_b.json"
    p.write_text(json.dumps(payload), encoding="utf-8")
    return p


def _write_py(tmp_path: Path, name: str, content: str) -> Path:
    p = tmp_path / name
    p.write_text(content, encoding="utf-8")
    return p


# ---------------------------------------------------------------------------
# Helper: compute expected BPE score for a token ID
# ---------------------------------------------------------------------------


def _expected_bpe(
    token_id: int,
    model_a: dict[int, int],
    total_a: int,
    model_b: dict[int, int],
    total_b: int,
) -> float:
    return math.log(model_b.get(token_id, 0) / total_b + _EPSILON) - math.log(
        model_a.get(token_id, 0) / total_a + _EPSILON
    )


# ---------------------------------------------------------------------------
# Unit tests for _is_meaningful_token
# ---------------------------------------------------------------------------


def test_meaningful_token_long_alnum() -> None:
    assert _is_meaningful_token("token_abc") is True


def test_meaningful_token_short() -> None:
    assert _is_meaningful_token("ab") is False


def test_meaningful_token_no_alnum() -> None:
    assert _is_meaningful_token("___") is False


def test_meaningful_token_mixed() -> None:
    assert _is_meaningful_token("Ġabc") is True  # starts with Ġ but has alnum


# ---------------------------------------------------------------------------
# Stage 1 fires → flagged=True, reason="import"
# ---------------------------------------------------------------------------


def test_stage1_fires_on_foreign_import(tmp_path: Path) -> None:
    # model_a has only "faker"
    ma_file = _write_py(tmp_path, "a.py", "import faker\n")
    model_b_path = _make_model_b(tmp_path, {1: 50, 2: 50})

    tok = _FakeTok(
        {
            "import faker\n": [1],
            "from mimesis import Person\n": [1],
        }
    )

    scorer = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["import faker\n"],
        _tokenizer=tok,
    )

    result = scorer.score_hunk("from mimesis import Person\n")
    assert result["flagged"] is True
    assert result["reason"] == "import"
    assert result["import_score"] >= 1.0


# ---------------------------------------------------------------------------
# Stage 1=0, BPE > threshold → flagged=True, reason="bpe"
# ---------------------------------------------------------------------------


def test_stage2_fires_on_high_bpe(tmp_path: Path) -> None:
    # model_a has "faker", hunk has no foreign import
    ma_file = _write_py(tmp_path, "a.py", "import faker\n")

    # token 1: heavy in model_a, rare in model_b → low BPE score (calibration)
    # token 2: absent in model_a, heavy in model_b → high BPE score (break hunk)
    model_b_path = _make_model_b(tmp_path, {1: 1, 2: 99})

    # model_a file encodes to [1, 1, 1] (token 1 three times)
    # calibration hunk encodes to [1] → threshold = low score
    # break hunk encodes to [2] → bpe_score >> threshold
    tok = _FakeTok(
        {
            "import faker\n": [1, 1, 1],
            "calibration\n": [1],
            "high_bpe_hunk\n": [2],
        }
    )

    scorer = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["calibration\n"],
        _tokenizer=tok,
    )

    result = scorer.score_hunk("high_bpe_hunk\n")
    assert result["flagged"] is True
    assert result["reason"] == "bpe"
    assert result["import_score"] == 0.0


# ---------------------------------------------------------------------------
# Stage 1=0, BPE ≤ threshold → flagged=False, reason="none"
# ---------------------------------------------------------------------------


def test_no_stage_fires_below_threshold(tmp_path: Path) -> None:
    ma_file = _write_py(tmp_path, "a.py", "import faker\n")
    model_b_path = _make_model_b(tmp_path, {1: 1, 2: 99})

    tok = _FakeTok(
        {
            "import faker\n": [1, 1, 1],
            "calibration\n": [1],
            # same token as calibration → bpe_score = threshold, NOT > threshold
            "same_as_cal\n": [1],
        }
    )

    scorer = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["calibration\n"],
        _tokenizer=tok,
    )

    result = scorer.score_hunk("same_as_cal\n")
    assert result["flagged"] is False
    assert result["reason"] == "none"


# ---------------------------------------------------------------------------
# Threshold equals max over provided calibration set
# ---------------------------------------------------------------------------


def test_threshold_is_max_of_calibration(tmp_path: Path) -> None:
    ma_file = _write_py(tmp_path, "a.py", "import faker\n")

    # token 1: low score, token 2: higher score, token 3: highest score
    model_b_path = _make_model_b(tmp_path, {1: 1, 2: 10, 3: 89})

    tok = _FakeTok(
        {
            "import faker\n": [1],
            "cal_low\n": [1],
            "cal_mid\n": [2],
            "cal_high\n": [3],
        }
    )

    # calibration = three hunks with different scores
    scorer = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["cal_low\n", "cal_mid\n", "cal_high\n"],
        _tokenizer=tok,
    )

    # Expected threshold: max of scores for tokens 1, 2, 3
    model_b: dict[int, int] = {1: 1, 2: 10, 3: 89}
    total_b = 100
    # model_a after encoding "import faker\n" → [1]
    model_a: dict[int, int] = {1: 1}
    total_a = 1

    score_1 = _expected_bpe(1, model_a, total_a, model_b, total_b)
    score_2 = _expected_bpe(2, model_a, total_a, model_b, total_b)
    score_3 = _expected_bpe(3, model_a, total_a, model_b, total_b)
    expected_threshold = max(score_1, score_2, score_3)

    assert abs(scorer.bpe_threshold - expected_threshold) < 1e-9


# ---------------------------------------------------------------------------
# bpe_score is always populated even when Stage 1 fires
# ---------------------------------------------------------------------------


def test_bpe_score_always_populated(tmp_path: Path) -> None:
    ma_file = _write_py(tmp_path, "a.py", "import faker\n")
    model_b_path = _make_model_b(tmp_path, {1: 50, 2: 50})

    tok = _FakeTok(
        {
            "import faker\n": [1],
            "calibration\n": [1],
            "from mimesis import Person\n": [2],
        }
    )

    scorer = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["calibration\n"],
        _tokenizer=tok,
    )

    result = scorer.score_hunk("from mimesis import Person\n")
    # Stage 1 fired, but bpe_score should still be present and a finite float
    assert result["reason"] == "import"
    assert isinstance(result["bpe_score"], float)
    assert math.isfinite(result["bpe_score"])


# ---------------------------------------------------------------------------
# n_calibration is recorded correctly
# ---------------------------------------------------------------------------


def test_n_calibration_stored(tmp_path: Path) -> None:
    ma_file = _write_py(tmp_path, "a.py", "import faker\n")
    model_b_path = _make_model_b(tmp_path, {1: 100})

    tok = _FakeTok({"import faker\n": [1], "c1\n": [1], "c2\n": [1], "c3\n": [1]})

    scorer = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["c1\n", "c2\n", "c3\n"],
        _tokenizer=tok,
    )

    assert scorer.n_calibration == 3


# ---------------------------------------------------------------------------
# extract_imports unit tests
# ---------------------------------------------------------------------------


def test_extract_imports_valid_python() -> None:
    source = "import os\nimport sys\n\ndef foo():\n    pass\n"
    result = extract_imports(source)
    assert result == "import os\nimport sys"


def test_extract_imports_from_import() -> None:
    source = "from pathlib import Path\nimport json\n\nclass Foo:\n    pass\n"
    result = extract_imports(source)
    assert result == "from pathlib import Path\nimport json"


def test_extract_imports_with_leading_docstring() -> None:
    # Module docstring before imports — imports must still be collected
    source = '"""Module docstring."""\nimport os\nfrom sys import path\n'
    result = extract_imports(source)
    assert "import os" in result
    assert "from sys import path" in result


def test_extract_imports_syntax_error_fallback() -> None:
    # Invalid Python — should fall back to regex
    source = "import os\nfrom sys import path\ndef broken(:\n    pass\n"
    result = extract_imports(source)
    assert "import os" in result
    assert "from sys import path" in result


def test_extract_imports_empty_source() -> None:
    assert extract_imports("") == ""


# ---------------------------------------------------------------------------
# Stage 1 detects foreign imports via file_source even if not in hunk
# ---------------------------------------------------------------------------


def test_stage1_detects_foreign_import_via_file_source(tmp_path: Path) -> None:
    """Stage 1 should flag a hunk when the foreign import is in the file header
    (file_source) but not in the hunk_content itself."""
    # model_a has only "faker"
    ma_file = _write_py(tmp_path, "a.py", "import faker\n")
    model_b_path = _make_model_b(tmp_path, {1: 50, 2: 50})

    # file_source contains "from mimesis import Person" (foreign import for model_a)
    # hunk_content is just a plain function body with no imports
    file_source = "from mimesis import Person\n\ndef generate():\n    return Person().name()\n"
    hunk_content = "def generate():\n    return Person().name()\n"

    tok = _FakeTok(
        {
            "import faker\n": [1],
            "calibration\n": [1],
            # stage1_input = extract_imports(file_source) + "\n" + hunk_content
            "from mimesis import Person\n\ndef generate():\n    return Person().name()\n": [1],
            hunk_content: [1],
            # The actual stage1_input passed to ImportGraphScorer:
            "from mimesis import Person" + "\n" + hunk_content: [1],
        }
    )

    scorer = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["calibration\n"],
        _tokenizer=tok,
    )

    result = scorer.score_hunk(hunk_content, file_source=file_source)
    # ImportGraphScorer parses text — it sees "from mimesis import Person" in stage1_input
    assert result["flagged"] is True
    assert result["reason"] == "import"
    assert result["import_score"] >= 1.0


# ---------------------------------------------------------------------------
# Stage 2 score is invariant to file size (file_source does not affect BPE)
# ---------------------------------------------------------------------------


def test_stage2_bpe_invariant_to_file_size(tmp_path: Path) -> None:
    """Stage 2 BPE score must be identical regardless of how much code is in
    file_source — the file prefix must not affect BPE scoring."""
    ma_file = _write_py(tmp_path, "a.py", "import faker\n")
    model_b_path = _make_model_b(tmp_path, {1: 50, 2: 50})

    hunk_content = "def simple():\n    return 42\n"
    # A large, unrelated file prefix
    large_file_source = ("# unrelated\n" * 1000) + hunk_content

    tok = _FakeTok(
        {
            "import faker\n": [1],
            "calibration\n": [1],
            hunk_content: [2],
            # large_file_source would encode differently but it should NOT be
            # passed to _bpe_score at all
        }
    )

    scorer = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["calibration\n"],
        _tokenizer=tok,
    )

    score_without_file = scorer.score_hunk(hunk_content)
    score_with_file = scorer.score_hunk(hunk_content, file_source=large_file_source)

    # BPE score must be identical — file_source must not affect Stage 2
    assert score_without_file["bpe_score"] == score_with_file["bpe_score"]
