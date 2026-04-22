# engine/argot/research/signal/phase14/scorers/test_sequential_import_bpe_scorer.py
"""Tests for SequentialImportBpeScorer."""

from __future__ import annotations

import json
import math
from pathlib import Path

from argot.research.signal.phase14.parsers import Parser
from argot.research.signal.phase14.scorers.sequential_import_bpe_scorer import (
    SequentialImportBpeScorer,
    _blank_prose_lines,
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
    (file_source) but not in the hunk_content itself.

    Crucially, hunk_content is an incomplete code fragment (mid-block slice) that
    would produce a SyntaxError if concatenated with the import block.  The split
    approach — parse extract_imports(file_source) and hunk_content separately —
    must still detect the foreign import from file_source.
    """
    # model_a has only "faker"
    ma_file = _write_py(tmp_path, "a.py", "import faker\n")
    model_b_path = _make_model_b(tmp_path, {1: 50, 2: 50})

    # file_source contains "from voluptuous import Schema" (foreign import for model_a)
    # hunk_content is a mid-block slice — concatenating it with the import line
    # produces invalid Python (SyntaxError), so the old approach would have needed
    # a regex fallback which caused docstring false positives.
    file_source = (
        "from voluptuous import Schema\n\ndef validate(data):\n    schema = Schema({str: int})\n"
    )
    # Mid-block fragment: indented code with no closing block — invalid if ast.parse'd
    # together with the import line above
    hunk_content = "    schema = Schema({str: int})\n    return schema(data)\n"

    tok = _FakeTok(
        {
            "import faker\n": [1],
            "calibration\n": [1],
            hunk_content: [1],
        }
    )

    scorer = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["calibration\n"],
        _tokenizer=tok,
    )

    result = scorer.score_hunk(hunk_content, file_source=file_source)
    # Stage 1 must detect "voluptuous" from file_source even though the concatenated
    # string would be invalid Python — the split parse approach handles this correctly.
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


# ---------------------------------------------------------------------------
# Fake parser for prose-masking tests
# ---------------------------------------------------------------------------


class _FakeParser:
    """Parser that returns a fixed frozenset of prose line numbers for any source."""

    def __init__(self, prose_lines: frozenset[int]) -> None:
        self._prose_lines = prose_lines

    def prose_line_ranges(self, src: str) -> frozenset[int]:
        return self._prose_lines


# Verify _FakeParser satisfies the Parser protocol
_: Parser = _FakeParser(frozenset())


# ---------------------------------------------------------------------------
# Test 1: Masking drops BPE score when prose lines are blanked
# ---------------------------------------------------------------------------


def test_prose_masking_lowers_bpe_score(tmp_path: Path) -> None:
    """score_hunk with hunk_start_line/hunk_end_line should return a lower bpe_score
    than without, when the hunk contains high-scoring prose tokens that get blanked."""
    ma_file = _write_py(tmp_path, "a.py", "import faker\n")

    # token 1: absent in model_a, heavy in model_b → very high BPE score (prose tokens)
    # token 2: heavy in model_a, rare in model_b → low BPE score (code tokens)
    model_b_path = _make_model_b(tmp_path, {1: 90, 2: 10})

    # Hunk source: line 1 is prose (high-scoring token), line 2 is code (low-scoring token).
    # The _FakeParser marks line 1 as prose within the file (file line = hunk_start_line + 0).
    # _blank_prose_lines will blank line 1 in hunk_content, leaving only line 2 for BPE.
    hunk_content = "docstring text here\ncode_line\n"
    # Blanked version: line 1 becomes "\n", only line 2 ("code_line\n") remains
    blanked_hunk = "\ncode_line\n"
    file_source = "# file header\n" + hunk_content  # hunk starts at line 2

    tok = _FakeTok(
        {
            "import faker\n": [2, 2],
            "calibration\n": [2],  # calibration hunk → low threshold (no prose, returned as-is)
            hunk_content: [1],  # raw hunk → high-scoring prose token
            blanked_hunk: [2],  # blanked hunk → low-scoring code token
        }
    )

    # hunk starts at line 2 in the file (line 1 is "# file header\n")
    # prose_line_ranges returns {2} for any source → line 2 in the file = line 1 in hunk
    fake_parser = _FakeParser(frozenset({2}))

    scorer = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["calibration\n"],
        parser=fake_parser,
        _tokenizer=tok,
    )

    # Without masking (no line kwargs): raw hunk uses token 1 (high BPE)
    score_raw = scorer.score_hunk(hunk_content, file_source=file_source)
    # With masking: blanked hunk uses token 2 (low BPE)
    score_masked = scorer.score_hunk(
        hunk_content,
        file_source=file_source,
        hunk_start_line=2,
        hunk_end_line=3,
    )

    assert score_masked["bpe_score"] < score_raw["bpe_score"]


# ---------------------------------------------------------------------------
# Test 2: Symmetric calibration — threshold is lower when cal hunks have prose
# ---------------------------------------------------------------------------


def test_symmetric_calibration_lowers_threshold(tmp_path: Path) -> None:
    """When calibration hunks contain prose tokens that score high in model_b,
    blanking them symmetrically should result in a lower bpe_threshold compared
    to a scorer that does NOT blank during calibration (i.e. uses default parser
    but with a fake parser that returns no prose lines)."""
    ma_file = _write_py(tmp_path, "a.py", "import faker\n")

    # token 1: absent in model_a, very heavy in model_b → high BPE score
    # token 2: in model_a, moderate in model_b → lower BPE score
    model_b_path = _make_model_b(tmp_path, {1: 90, 2: 10})

    # Calibration hunk: line 1 is prose (token 1), line 2 is code (token 2)
    cal_hunk = "docstring line\ncode line\n"
    blanked_cal_hunk = "\ncode line\n"

    tok = _FakeTok(
        {
            "import faker\n": [2, 2],
            cal_hunk: [1],         # raw cal hunk → high-BPE token (prose)
            blanked_cal_hunk: [2], # blanked cal hunk → lower-BPE token (code)
        }
    )

    # parser that marks line 1 of any source as prose → blanks first line of cal hunk
    prose_parser = _FakeParser(frozenset({1}))
    # parser that returns no prose → calibration uses raw hunk (token 1)
    no_prose_parser = _FakeParser(frozenset())

    scorer_with_prose = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=[cal_hunk],
        parser=prose_parser,
        _tokenizer=tok,
    )
    scorer_without_prose = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=[cal_hunk],
        parser=no_prose_parser,
        _tokenizer=tok,
    )

    # With prose blanking, threshold is computed on blanked_cal_hunk (token 2 → lower score)
    # Without prose blanking, threshold is computed on raw cal_hunk (token 1 → higher score)
    assert scorer_with_prose.bpe_threshold < scorer_without_prose.bpe_threshold


# ---------------------------------------------------------------------------
# Test 3: Back-compat — no masking when kwargs are absent or partial
# ---------------------------------------------------------------------------


def test_back_compat_no_masking_without_line_kwargs(tmp_path: Path) -> None:
    """score_hunk without hunk_start_line/hunk_end_line must return the same result
    as before the prose-masking change, regardless of whether file_source is provided."""
    ma_file = _write_py(tmp_path, "a.py", "import faker\n")
    model_b_path = _make_model_b(tmp_path, {1: 50, 2: 50})

    hunk_content = "def foo():\n    return 42\n"
    file_source = "import os\n" + hunk_content

    tok = _FakeTok(
        {
            "import faker\n": [1],
            "calibration\n": [1],
            hunk_content: [2],
        }
    )

    # parser that marks everything as prose — but it should NOT be invoked when
    # hunk_start_line/hunk_end_line are absent
    all_prose_parser = _FakeParser(frozenset({1, 2, 3, 4, 5}))

    scorer = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["calibration\n"],
        parser=all_prose_parser,
        _tokenizer=tok,
    )

    # Call without any kwargs — no masking
    score_no_kwargs = scorer.score_hunk(hunk_content)
    # Call with file_source only (no line kwargs) — no masking
    score_file_only = scorer.score_hunk(hunk_content, file_source=file_source)
    # Call with hunk_start_line only (missing hunk_end_line) — no masking
    score_partial = scorer.score_hunk(
        hunk_content, file_source=file_source, hunk_start_line=2
    )

    # All three should produce identical BPE scores (no masking applied)
    assert score_no_kwargs["bpe_score"] == score_file_only["bpe_score"]
    assert score_no_kwargs["bpe_score"] == score_partial["bpe_score"]


# ---------------------------------------------------------------------------
# Auto-generated file short-circuit
# ---------------------------------------------------------------------------


def test_auto_generated_short_circuits_both_stages(tmp_path: Path) -> None:
    """score_hunk must return flagged=False, reason='auto_generated' without
    invoking Stage 1 or Stage 2 when file_source is an auto-generated file."""
    # model_a has only "faker"; hunk has a foreign import that would trigger Stage 1
    ma_file = _write_py(tmp_path, "a.py", "import faker\n")
    model_b_path = _make_model_b(tmp_path, {1: 50, 2: 50})

    # hunk_content contains a foreign import — would normally fire Stage 1
    hunk_content = "from voluptuous import Schema\n"
    # file_source has an auto-gen marker in the header
    file_source = "# auto-generated — do not edit\nfrom voluptuous import Schema\n"

    tok = _FakeTok(
        {
            "import faker\n": [1],
            "calibration\n": [1],
            hunk_content: [2],
        }
    )

    scorer = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["calibration\n"],
        _tokenizer=tok,
    )

    result = scorer.score_hunk(hunk_content, file_source=file_source)
    assert result["flagged"] is False
    assert result["reason"] == "auto_generated"
    assert result["import_score"] == 0.0
    assert result["bpe_score"] == 0.0


def test_auto_generated_only_fires_when_file_source_provided(tmp_path: Path) -> None:
    """Without file_source, auto-gen detection is skipped and Stage 1 may fire."""
    ma_file = _write_py(tmp_path, "a.py", "import faker\n")
    model_b_path = _make_model_b(tmp_path, {1: 50, 2: 50})

    # hunk_content has a foreign import — Stage 1 should catch it
    hunk_content = "from voluptuous import Schema\n"

    tok = _FakeTok(
        {
            "import faker\n": [1],
            "calibration\n": [1],
            hunk_content: [2],
        }
    )

    scorer = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["calibration\n"],
        _tokenizer=tok,
    )

    # No file_source → auto-gen check skipped → Stage 1 catches foreign import
    result = scorer.score_hunk(hunk_content)
    assert result["reason"] == "import"
    assert result["flagged"] is True


# ---------------------------------------------------------------------------
# exclude_data_dominant (fix9)
# ---------------------------------------------------------------------------

# A multiline list assignment that makes >65% of lines data literals.
_DD_CONTENT = "x = [\n    'a',\n    'b',\n    'c',\n    'd',\n    'e',\n]\n"


def test_exclude_data_dominant_default_excludes_dd_files(tmp_path: Path) -> None:
    """Default exclude_data_dominant=True must remove data-dominant files from model A.

    The data-dominant file encodes to unique token IDs [7, 8].  After exclusion
    those token IDs must be absent from _model_a.
    """
    normal_file = _write_py(tmp_path, "normal.py", "import os\n")
    dd_file = _write_py(tmp_path, "dd.py", _DD_CONTENT)
    model_b_path = _make_model_b(tmp_path, {1: 50, 2: 50, 7: 1, 8: 1})

    tok = _FakeTok(
        {
            "import os\n": [1, 2],
            _DD_CONTENT: [7, 8],
            "calibration\n": [1],
        }
    )

    scorer = SequentialImportBpeScorer(
        model_a_files=[normal_file, dd_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["calibration\n"],
        _tokenizer=tok,
    )

    assert 7 not in scorer._model_a, "data-dominant file token should be excluded from model A"
    assert 8 not in scorer._model_a, "data-dominant file token should be excluded from model A"
    assert 1 in scorer._model_a, "normal file token should remain in model A"


def test_exclude_data_dominant_opt_out_includes_all_files(tmp_path: Path) -> None:
    """exclude_data_dominant=False must include all files — reproduces pre-fix9 behaviour."""
    normal_file = _write_py(tmp_path, "normal.py", "import os\n")
    dd_file = _write_py(tmp_path, "dd.py", _DD_CONTENT)
    model_b_path = _make_model_b(tmp_path, {1: 50, 2: 50, 7: 1, 8: 1})

    tok = _FakeTok(
        {
            "import os\n": [1, 2],
            _DD_CONTENT: [7, 8],
            "calibration\n": [1],
        }
    )

    scorer = SequentialImportBpeScorer(
        model_a_files=[normal_file, dd_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["calibration\n"],
        _tokenizer=tok,
        exclude_data_dominant=False,
    )

    assert 7 in scorer._model_a, "data-dominant file token should be in model A when opt-out"
    assert 8 in scorer._model_a, "data-dominant file token should be in model A when opt-out"


def test_exclude_data_dominant_empty_corpus_raises(tmp_path: Path) -> None:
    """exclude_data_dominant=True must raise ValueError when all files are data-dominant."""
    dd_file = _write_py(tmp_path, "dd.py", _DD_CONTENT)
    model_b_path = _make_model_b(tmp_path, {7: 50, 8: 50})

    tok = _FakeTok({_DD_CONTENT: [7, 8], "calibration\n": [7]})

    import pytest

    with pytest.raises(ValueError, match="empty corpus"):
        SequentialImportBpeScorer(
            model_a_files=[dd_file],
            bpe_model_b_path=model_b_path,
            calibration_hunks=["calibration\n"],
            _tokenizer=tok,
        )


def test_non_auto_generated_file_proceeds_normally(tmp_path: Path) -> None:
    """A regular file_source (no markers) must not trigger auto_generated short-circuit."""
    ma_file = _write_py(tmp_path, "a.py", "import faker\n")
    model_b_path = _make_model_b(tmp_path, {1: 50, 2: 50})

    hunk_content = "from voluptuous import Schema\n"
    # Normal file header — no auto-gen markers
    file_source = "# Hand-written module.\nfrom voluptuous import Schema\n"

    tok = _FakeTok(
        {
            "import faker\n": [1],
            "calibration\n": [1],
            hunk_content: [2],
        }
    )

    scorer = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=model_b_path,
        calibration_hunks=["calibration\n"],
        _tokenizer=tok,
    )

    result = scorer.score_hunk(hunk_content, file_source=file_source)
    assert result["reason"] == "import"
    assert result["flagged"] is True
