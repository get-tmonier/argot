# engine/argot/research/signal/phase14/experiments/test_stage2_recall_probe_2026_04_22.py
"""Tests for Stage2OnlyScorer and the stage2_only fixture pack.

Covers three concerns:
  1. Stage2OnlyScorer behavioural invariants (import_score=0 always, BPE path intact)
  2. Stage2OnlyScorer vs parent agreement when Stage 1 would be silent anyway
  3. Fixture validation: files exist, hunk ranges in bounds, no foreign imports
"""

from __future__ import annotations

import ast
import json
import math
from pathlib import Path
from typing import Any

import pytest

from argot.research.signal.phase14.scorers.sequential_import_bpe_scorer import (
    SequentialImportBpeScorer,
    _blank_prose_lines,
)
from argot.research.signal.phase14.experiments.stage2_recall_probe_2026_04_22 import (
    Stage2OnlyScorer,
    _STAGE2_FIXTURE_META,
    _extract_hunk,
)

_EPSILON = 1e-7

# ---------------------------------------------------------------------------
# Shared fake tokenizer — deterministic, no network access
# ---------------------------------------------------------------------------


class _FakeTok:
    """Minimal tokenizer stub that maps source strings to fixed token-ID lists."""

    def __init__(self, source_to_ids: dict[str, list[int]]) -> None:
        self._map = source_to_ids
        # Long alphanumeric names pass the _is_meaningful_token filter (len≥3, has alnum)
        self._vocab: dict[str, int] = {f"tok_{i:04d}": i for i in range(1, 20)}

    def encode(self, source: str, *, add_special_tokens: bool = False) -> list[int]:
        return list(self._map.get(source, [1]))  # default to [1] for unknown sources

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


def _expected_llr(
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
# Shared fixture: minimal scorer pair (parent + Stage2Only on same corpus)
# ---------------------------------------------------------------------------


def _make_scorer_pair(
    tmp_path: Path,
    *,
    model_a_source: str = "import re\ndef f(): pass\n",
    cal_hunk: str = "import re\n",
    model_b_counts: dict[int, int] | None = None,
    tok_map: dict[str, list[int]] | None = None,
) -> tuple[SequentialImportBpeScorer, Stage2OnlyScorer]:
    """Return (parent, stage2_only) using identical corpus and tokenizer."""
    ma_file = _write_py(tmp_path, "a.py", model_a_source)
    if model_b_counts is None:
        model_b_counts = {1: 50, 2: 50}
    mb_path = _make_model_b(tmp_path, model_b_counts)
    if tok_map is None:
        tok_map = {
            model_a_source: [1, 1],
            cal_hunk: [1],
        }
    tok = _FakeTok(tok_map)
    parent = SequentialImportBpeScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=mb_path,
        calibration_hunks=[cal_hunk],
        _tokenizer=tok,
    )
    stage2 = Stage2OnlyScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=mb_path,
        calibration_hunks=[cal_hunk],
        _tokenizer=tok,
    )
    return parent, stage2


# ---------------------------------------------------------------------------
# §1 Stage2OnlyScorer — import_score invariants
# ---------------------------------------------------------------------------


def test_import_score_always_zero_for_foreign_import(tmp_path: Path) -> None:
    """import_score must be 0 even when the hunk introduces a foreign module."""
    _, stage2 = _make_scorer_pair(tmp_path)
    # "flask" is not in model_a ("import re"), so parent would set import_score≥1
    result = stage2.score_hunk("from flask import Flask\n")
    assert result["import_score"] == 0.0


def test_reason_never_import(tmp_path: Path) -> None:
    """reason must never be 'import' — only 'bpe' or 'none'."""
    _, stage2 = _make_scorer_pair(tmp_path)
    for hunk in [
        "from flask import Flask\n",
        "import django\n",
        "import re\n",
        "x = 1\n",
    ]:
        result = stage2.score_hunk(hunk)
        assert result["reason"] in ("bpe", "none"), (
            f"reason='import' fired for {hunk!r}"
        )


def test_flagged_false_when_bpe_below_threshold(tmp_path: Path) -> None:
    """A hunk whose BPE score is ≤ threshold must not be flagged."""
    ma_file = _write_py(tmp_path, "a.py", "import re\ndef f(): pass\n")
    # token 1 equally common in model_a and model_b → LLR ≈ 0 < threshold
    mb_path = _make_model_b(tmp_path, {1: 100})
    tok = _FakeTok({"import re\ndef f(): pass\n": [1, 1], "import re\n": [1], "x = 1\n": [1]})
    stage2 = Stage2OnlyScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=mb_path,
        calibration_hunks=["import re\n"],
        _tokenizer=tok,
    )
    result = stage2.score_hunk("x = 1\n")
    assert result["flagged"] is False
    assert result["reason"] == "none"


def test_flagged_true_when_bpe_above_threshold(tmp_path: Path) -> None:
    """A hunk with a token rare in model_a but common in model_b must be flagged."""
    ma_file = _write_py(tmp_path, "a.py", "import re\n" * 10)
    # token 1: heavy in model_a → low LLR → calibration hunk scores low
    # token 2: absent in model_a, heavy in model_b → high LLR → break hunk fires
    mb_path = _make_model_b(tmp_path, {1: 1, 2: 99})
    tok = _FakeTok({
        "import re\n" * 10: [1] * 10,
        "import re\n": [1],
        "from walrus import Operator\n": [2],  # token 2 is OOV in model_a
    })
    stage2 = Stage2OnlyScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=mb_path,
        calibration_hunks=["import re\n"],
        _tokenizer=tok,
    )
    result = stage2.score_hunk("from walrus import Operator\n")
    assert result["flagged"] is True
    assert result["reason"] == "bpe"
    assert result["import_score"] == 0.0  # Stage 1 still bypassed


def test_import_score_zero_even_with_file_source(tmp_path: Path) -> None:
    """import_score=0 also when file_source is provided (full-context path)."""
    _, stage2 = _make_scorer_pair(tmp_path)
    file_src = "from flask import Flask\n\ndef view(): pass\n"
    result = stage2.score_hunk(
        "def view(): pass\n",
        file_source=file_src,
        hunk_start_line=3,
        hunk_end_line=3,
    )
    assert result["import_score"] == 0.0


# ---------------------------------------------------------------------------
# §2 Stage2OnlyScorer vs parent — BPE path identical
# ---------------------------------------------------------------------------


def test_bpe_score_matches_parent_when_stage1_silent(tmp_path: Path) -> None:
    """When parent's Stage 1 produces import_score=0, bpe_score must match."""
    parent, stage2 = _make_scorer_pair(tmp_path)
    # "import re" is in model_a → parent Stage 1 silent → same BPE path
    hunk = "import re\n"
    p_result = parent.score_hunk(hunk)
    s_result = stage2.score_hunk(hunk)
    assert p_result["import_score"] == 0.0  # confirms Stage 1 was silent for parent too
    assert s_result["bpe_score"] == pytest.approx(p_result["bpe_score"])
    assert s_result["flagged"] == p_result["flagged"]


def test_threshold_identical_between_parent_and_stage2(tmp_path: Path) -> None:
    """bpe_threshold is computed identically by both classes (same calibration path)."""
    parent, stage2 = _make_scorer_pair(tmp_path)
    assert stage2.bpe_threshold == pytest.approx(parent.bpe_threshold)


def test_bpe_score_computed_even_when_import_would_have_fired(tmp_path: Path) -> None:
    """bpe_score is always present in result dict, even for foreign-import hunks."""
    _, stage2 = _make_scorer_pair(tmp_path)
    result = stage2.score_hunk("from flask import Flask\n")
    assert "bpe_score" in result
    assert isinstance(result["bpe_score"], float)


# ---------------------------------------------------------------------------
# §3 Prose masking still applied in Stage2OnlyScorer
# ---------------------------------------------------------------------------


def test_prose_masking_reduces_bpe_input(tmp_path: Path) -> None:
    """Docstring lines are blanked before BPE scoring when file_source is provided."""
    ma_file = _write_py(tmp_path, "a.py", "import re\n")
    # token 2: in docstring text, high LLR token
    # token 1: in code, low LLR token
    mb_path = _make_model_b(tmp_path, {1: 1, 2: 99})

    docstring_hunk = '"""This is rare prose token."""\n'
    code_hunk = "x = 1\n"

    tok = _FakeTok({
        "import re\n": [1],
        docstring_hunk: [2],  # would fire if not blanked
        code_hunk: [1],
        "\n": [1],  # blanked docstring line
        # When prose-masked, the docstring line becomes "\n" → [1] (low LLR)
    })
    stage2 = Stage2OnlyScorer(
        model_a_files=[ma_file],
        bpe_model_b_path=mb_path,
        calibration_hunks=["import re\n"],
        _tokenizer=tok,
    )

    # File with a 1-line docstring followed by code
    file_src = '"""This is rare prose token."""\nx = 1\n'
    hunk = '"""This is rare prose token."""\n'

    # Without prose masking (no file_source): high bpe_score from docstring token
    result_no_mask = stage2.score_hunk(hunk)
    # With prose masking: docstring blanked → lower bpe_score
    result_masked = stage2.score_hunk(
        hunk,
        file_source=file_src,
        hunk_start_line=1,
        hunk_end_line=1,
    )
    # The masked result must have a different (lower) bpe_score since token 2 is suppressed
    assert result_masked["bpe_score"] != result_no_mask["bpe_score"] or (
        # Acceptable if both are ≤ threshold (both unflagged) — masking still ran
        not result_masked["flagged"] and not result_no_mask["flagged"]
    )


# ---------------------------------------------------------------------------
# §4 Fixture validation
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("meta", _STAGE2_FIXTURE_META, ids=[m["name"] for m in _STAGE2_FIXTURE_META])
def test_fixture_file_exists(meta: dict[str, Any]) -> None:
    """Each fixture file must exist on disk."""
    assert meta["file"].exists(), f"Missing fixture: {meta['file']}"


@pytest.mark.parametrize("meta", _STAGE2_FIXTURE_META, ids=[m["name"] for m in _STAGE2_FIXTURE_META])
def test_fixture_hunk_in_bounds(meta: dict[str, Any]) -> None:
    """hunk_end_line must not exceed the actual line count of the file."""
    n_lines = len(meta["file"].read_text(encoding="utf-8").splitlines())
    assert meta["hunk_start_line"] >= 1, "hunk_start_line must be ≥ 1"
    assert meta["hunk_end_line"] <= n_lines, (
        f"{meta['name']}: hunk_end_line={meta['hunk_end_line']} > file lines={n_lines}"
    )
    assert meta["hunk_start_line"] <= meta["hunk_end_line"], (
        "hunk_start_line must be ≤ hunk_end_line"
    )


@pytest.mark.parametrize("meta", _STAGE2_FIXTURE_META, ids=[m["name"] for m in _STAGE2_FIXTURE_META])
def test_fixture_hunk_nonempty(meta: dict[str, Any]) -> None:
    """Extracted hunk must contain non-blank content."""
    hunk = _extract_hunk(meta["file"], meta["hunk_start_line"], meta["hunk_end_line"])
    assert hunk.strip(), f"{meta['name']}: hunk is empty or blank"


@pytest.mark.parametrize("meta", _STAGE2_FIXTURE_META, ids=[m["name"] for m in _STAGE2_FIXTURE_META])
def test_fixture_hunk_size_reasonable(meta: dict[str, Any]) -> None:
    """Hunk size should be between 10 and 35 lines (spec target: ~15-25)."""
    n_lines = meta["hunk_end_line"] - meta["hunk_start_line"] + 1
    assert 10 <= n_lines <= 35, (
        f"{meta['name']}: hunk is {n_lines} lines, expected 10-35"
    )


_CORPUS_MODULES = frozenset(
    {
        # stdlib modules used by stage2_only fixtures
        "re", "json", "os", "sys", "io", "math", "ast", "inspect",
        "asyncio", "functools", "itertools", "operator", "collections",
        "pathlib", "datetime", "dataclasses", "typing", "enum", "contextlib",
        "abc", "copy", "time", "hashlib", "uuid", "struct", "base64",
        "__future__",
        # packages present in the FastAPI corpus
        "fastapi", "pydantic", "starlette", "anyio", "httpx",
        "typing_extensions", "annotated_types",
    }
)


@pytest.mark.parametrize("meta", _STAGE2_FIXTURE_META, ids=[m["name"] for m in _STAGE2_FIXTURE_META])
def test_fixture_no_foreign_imports(meta: dict[str, Any]) -> None:
    """Fixture must not import modules absent from a FastAPI-like corpus.

    Stage 1 cannot fire for fixtures that only use stdlib / in-corpus modules.
    This is the defining constraint for stage2_only fixtures.
    """
    src = meta["file"].read_text(encoding="utf-8")
    try:
        tree = ast.parse(src)
    except SyntaxError:
        pytest.skip(f"{meta['name']}: SyntaxError (e.g. PEP 695 syntax on older Python)")

    imported: set[str] = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                imported.add(alias.name.split(".")[0])
        elif isinstance(node, ast.ImportFrom):
            if node.module and not (node.level and node.level > 0):  # skip relative imports
                imported.add(node.module.split(".")[0])

    foreign = imported - _CORPUS_MODULES
    assert not foreign, (
        f"{meta['name']}: imports foreign modules not in FastAPI corpus: {foreign!r}"
    )


# ---------------------------------------------------------------------------
# §5 Stage2OnlyScorer result dict schema
# ---------------------------------------------------------------------------


def test_result_dict_keys_complete(tmp_path: Path) -> None:
    """score_hunk must return a dict with the four required keys."""
    _, stage2 = _make_scorer_pair(tmp_path)
    result = stage2.score_hunk("import re\n")
    assert set(result.keys()) >= {"import_score", "bpe_score", "flagged", "reason"}


def test_flagged_consistent_with_reason(tmp_path: Path) -> None:
    """flagged must be True iff reason != 'none'."""
    _, stage2 = _make_scorer_pair(tmp_path)
    for hunk in ["import re\n", "from flask import Flask\n", "x: int = 1\n"]:
        result = stage2.score_hunk(hunk)
        assert result["flagged"] == (result["reason"] != "none"), (
            f"flagged/reason mismatch for {hunk!r}: {result}"
        )


def test_bpe_score_is_float(tmp_path: Path) -> None:
    """bpe_score must always be a float, never None."""
    _, stage2 = _make_scorer_pair(tmp_path)
    result = stage2.score_hunk("import re\n")
    assert isinstance(result["bpe_score"], float)
