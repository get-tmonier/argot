"""Parity test: SequentialImportBpeScorer on fix10 fixtures within ±0.02 of reference.

Tests the bpe_score field from score_hunk() against fix10_reference.json.
Each domain uses the same model_A and calibration setup used to generate the reference.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

_CATALOG = Path(__file__).parent.parent / "acceptance" / "catalog"
_BPE_MODEL_B = Path(__file__).parent.parent / "scoring" / "bpe" / "generic_tokens_bpe.json"
_REFERENCE = Path(__file__).parent / "fixtures" / "fix10_reference.json"

_TOLERANCE = 0.02  # ±0.02 absolute tolerance


def _load_reference() -> dict[str, float]:
    return {k: float(v) for k, v in json.loads(_REFERENCE.read_text()).items()}


def _fastapi_scorer() -> tuple[SequentialImportBpeScorer, dict[str, Path]]:
    """Build fastapi scorer using control files as model_A."""
    fixtures_dir = _CATALOG / "fastapi" / "fixtures" / "default"
    model_a_files = sorted(fixtures_dir.glob("control_*.py"))
    manifest = json.loads((_CATALOG / "fastapi" / "manifest.json").read_text())
    fixture_map = {
        entry["name"]: fixtures_dir / Path(entry["file"]).name for entry in manifest["fixtures"]
    }
    # Calibration: 20 hunks from the control files (matches research setup: n_cal=20, seed=0)
    from argot.scoring.calibration.random_hunk_sampler import sample_hunks

    cal_hunks = sample_hunks(fixtures_dir, min(20, len(model_a_files)), seed=0)
    scorer = SequentialImportBpeScorer(
        model_a_files=model_a_files,
        bpe_model_b_path=_BPE_MODEL_B,
        calibration_hunks=cal_hunks,
    )
    return scorer, fixture_map


def _rich_scorer() -> tuple[SequentialImportBpeScorer, dict[str, Path]]:
    """Build rich scorer using rich/sources/model_a/ as model_A."""
    model_a_dir = _CATALOG / "rich" / "sources" / "model_a"
    model_a_files = sorted(model_a_dir.glob("*.py"))
    fixtures_dir = _CATALOG / "rich" / "fixtures"
    fixture_map = {p.stem: p for p in sorted(fixtures_dir.glob("*.py"))}
    from argot.scoring.calibration.random_hunk_sampler import sample_hunks

    n_cal = min(10, len(model_a_files) * 3)
    cal_hunks = sample_hunks(model_a_dir, max(1, n_cal), seed=0)
    scorer = SequentialImportBpeScorer(
        model_a_files=model_a_files,
        bpe_model_b_path=_BPE_MODEL_B,
        calibration_hunks=cal_hunks,
    )
    return scorer, fixture_map


def _faker_scorer() -> tuple[SequentialImportBpeScorer, dict[str, Path]]:
    """Build faker scorer using faker/sources/model_a/ as model_A."""
    model_a_dir = _CATALOG / "faker" / "sources" / "model_a"
    model_a_files = sorted(model_a_dir.glob("*.py"))
    fixtures_dir = _CATALOG / "faker" / "fixtures"
    fixture_map = {p.stem: p for p in sorted(fixtures_dir.glob("*.py"))}
    from argot.scoring.calibration.random_hunk_sampler import sample_hunks

    n_cal = min(159, max(1, len(model_a_files)))
    cal_hunks = sample_hunks(model_a_dir, n_cal, seed=0)
    scorer = SequentialImportBpeScorer(
        model_a_files=model_a_files,
        bpe_model_b_path=_BPE_MODEL_B,
        calibration_hunks=cal_hunks,
    )
    return scorer, fixture_map


@pytest.fixture(scope="module")
def reference() -> dict[str, float]:
    return _load_reference()


@pytest.fixture(scope="module")
def fastapi_scorer_and_fixtures() -> tuple[SequentialImportBpeScorer, dict[str, Path]]:
    return _fastapi_scorer()


@pytest.fixture(scope="module")
def rich_scorer_and_fixtures() -> tuple[SequentialImportBpeScorer, dict[str, Path]]:
    return _rich_scorer()


@pytest.fixture(scope="module")
def faker_scorer_and_fixtures() -> tuple[SequentialImportBpeScorer, dict[str, Path]]:
    return _faker_scorer()


def test_parity_fastapi(
    reference: dict[str, float],
    fastapi_scorer_and_fixtures: tuple[SequentialImportBpeScorer, dict[str, Path]],
) -> None:
    """All fastapi fixture bpe_scores within ±0.02 of reference."""
    scorer, fixture_map = fastapi_scorer_and_fixtures
    failures: list[str] = []
    checked = 0
    for key, ref_score in reference.items():
        if not key.startswith("fastapi/"):
            continue
        name = key[len("fastapi/") :]
        if name not in fixture_map:
            continue
        src = fixture_map[name].read_text(encoding="utf-8", errors="replace")
        result = scorer.score_hunk(src)
        got = result["bpe_score"]
        if abs(got - ref_score) > _TOLERANCE:
            failures.append(f"{key}: got {got:.4f}, ref {ref_score:.4f}, Δ={got - ref_score:.4f}")
        checked += 1
    assert checked > 0, "No fastapi fixtures matched"
    assert not failures, "Parity failures:\n" + "\n".join(failures)


def test_parity_rich(
    reference: dict[str, float],
    rich_scorer_and_fixtures: tuple[SequentialImportBpeScorer, dict[str, Path]],
) -> None:
    """All rich fixture bpe_scores within ±0.02 of reference."""
    scorer, fixture_map = rich_scorer_and_fixtures
    failures: list[str] = []
    checked = 0
    for key, ref_score in reference.items():
        if not key.startswith("rich/"):
            continue
        name = key[len("rich/") :]
        if name not in fixture_map:
            continue
        src = fixture_map[name].read_text(encoding="utf-8", errors="replace")
        result = scorer.score_hunk(src)
        got = result["bpe_score"]
        if abs(got - ref_score) > _TOLERANCE:
            failures.append(f"{key}: got {got:.4f}, ref {ref_score:.4f}, Δ={got - ref_score:.4f}")
        checked += 1
    assert checked > 0, "No rich fixtures matched"
    assert not failures, "Parity failures:\n" + "\n".join(failures)


def test_parity_faker(
    reference: dict[str, float],
    faker_scorer_and_fixtures: tuple[SequentialImportBpeScorer, dict[str, Path]],
) -> None:
    """All faker fixture bpe_scores within ±0.02 of reference."""
    scorer, fixture_map = faker_scorer_and_fixtures
    failures: list[str] = []
    checked = 0
    for key, ref_score in reference.items():
        if not key.startswith("faker/"):
            continue
        name = key[len("faker/") :]
        if name not in fixture_map:
            continue
        src = fixture_map[name].read_text(encoding="utf-8", errors="replace")
        result = scorer.score_hunk(src)
        got = result["bpe_score"]
        if abs(got - ref_score) > _TOLERANCE:
            failures.append(f"{key}: got {got:.4f}, ref {ref_score:.4f}, Δ={got - ref_score:.4f}")
        checked += 1
    assert checked > 0, "No faker fixtures matched"
    assert not failures, "Parity failures:\n" + "\n".join(failures)
