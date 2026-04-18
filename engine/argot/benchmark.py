from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import joblib  # type: ignore[import-untyped]
import numpy as np
import torch

from argot.jepa.encoder import TokenEncoder
from argot.jepa.model import JEPAArgot
from argot.jepa.predictor import ArgotPredictor
from argot.tokenize import language_for_path, tokenize_lines

BAND_ORDER: dict[str, int] = {"ok": 0, "unusual": 1, "suspicious": 2, "foreign": 3}

DEFAULT_FIXTURES_DIR = Path(__file__).parent / "benchmark_fixtures"


@dataclass(frozen=True, slots=True)
class FixtureSpec:
    name: str
    file: str
    hunk_start_line: int  # 1-indexed, inclusive
    hunk_end_line: int  # 1-indexed, exclusive
    min_band: str
    max_band: str
    rationale: str


@dataclass(frozen=True, slots=True)
class FixtureResult:
    name: str
    score: float
    percentile: float | None
    predicted_band: str
    min_band: str
    max_band: str
    passed: bool
    rationale: str


def score_to_band(score: float, threshold: float) -> str:
    if score <= threshold:
        return "ok"
    if score <= threshold + 0.3:
        return "unusual"
    if score <= threshold + 0.6:
        return "suspicious"
    return "foreign"


def band_in_range(predicted: str, min_band: str, max_band: str) -> bool:
    return BAND_ORDER[min_band] <= BAND_ORDER[predicted] <= BAND_ORDER[max_band]


def load_manifest(fixtures_dir: Path) -> list[FixtureSpec]:
    manifest_path = fixtures_dir / "manifest.json"
    data = json.loads(manifest_path.read_text())
    specs: list[FixtureSpec] = []
    for entry in data["fixtures"]:
        for band_field in ("min_band", "max_band"):
            if entry[band_field] not in BAND_ORDER:
                raise ValueError(
                    f"fixture {entry['name']}: {band_field}={entry[band_field]!r} "
                    f"must be one of {sorted(BAND_ORDER)}"
                )
        specs.append(
            FixtureSpec(
                name=entry["name"],
                file=entry["file"],
                hunk_start_line=entry["hunk_start_line"],
                hunk_end_line=entry["hunk_end_line"],
                min_band=entry["min_band"],
                max_band=entry["max_band"],
                rationale=entry["rationale"],
            )
        )
    return specs


def percentile_rank(value: float, distribution: list[float]) -> float:
    if not distribution:
        return 0.0
    arr = np.array(distribution)
    return float(np.mean(arr < value) * 100)


def _score_fixture_impl(
    spec: FixtureSpec,
    fixtures_dir: Path,
    vectorizer: Any,
    model: JEPAArgot,
) -> float | None:
    source_path = fixtures_dir / spec.file
    lang = language_for_path(spec.file)
    if lang is None:
        return None
    source_lines = source_path.read_text().splitlines()

    hunk_start = spec.hunk_start_line - 1
    hunk_end = spec.hunk_end_line - 1
    if hunk_start < 0 or hunk_end > len(source_lines) or hunk_start >= hunk_end:
        return None

    context_lines = 50
    before_start = max(0, hunk_start - context_lines)

    ctx_tokens = tokenize_lines(source_lines, lang, before_start, hunk_start)
    hunk_tokens = tokenize_lines(source_lines, lang, hunk_start, hunk_end)

    ctx_text = " ".join(t.text for t in ctx_tokens)
    hunk_text = " ".join(t.text for t in hunk_tokens)

    ctx_vec = torch.tensor(vectorizer.transform([ctx_text]).toarray(), dtype=torch.float32)
    hunk_vec = torch.tensor(vectorizer.transform([hunk_text]).toarray(), dtype=torch.float32)

    with torch.no_grad():
        return float(model.surprise(ctx_vec, hunk_vec).item())


def run_benchmark(
    specs: list[FixtureSpec],
    fixtures_dir: Path,
    score_fn: Callable[[FixtureSpec, Path], float | None],
    threshold: float,
    distribution: list[float],
) -> list[FixtureResult]:
    results: list[FixtureResult] = []
    for spec in specs:
        score = score_fn(spec, fixtures_dir)
        if score is None:
            continue
        predicted = score_to_band(score, threshold)
        pct = percentile_rank(score, distribution) if distribution else None
        results.append(
            FixtureResult(
                name=spec.name,
                score=score,
                percentile=pct,
                predicted_band=predicted,
                min_band=spec.min_band,
                max_band=spec.max_band,
                passed=band_in_range(predicted, spec.min_band, spec.max_band),
                rationale=spec.rationale,
            )
        )
    return results


def _build_distribution(
    dataset_path: Path,
    vectorizer: Any,
    model: JEPAArgot,
) -> list[float]:
    if not dataset_path.exists():
        return []
    records = [json.loads(line) for line in dataset_path.read_text().splitlines() if line.strip()]
    if not records:
        return []
    ctx_texts = [" ".join(t["text"] for t in r["context_before"]) for r in records]
    hunk_texts = [" ".join(t["text"] for t in r["hunk_tokens"]) for r in records]
    ctx_x = torch.tensor(vectorizer.transform(ctx_texts).toarray(), dtype=torch.float32)
    hunk_x = torch.tensor(vectorizer.transform(hunk_texts).toarray(), dtype=torch.float32)
    scores: list[float] = []
    with torch.no_grad():
        for i in range(len(records)):
            scores.append(float(model.surprise(ctx_x[i : i + 1], hunk_x[i : i + 1]).item()))
    return scores


def _print_report(results: list[FixtureResult], threshold: float) -> None:
    print(f"=== Benchmark (threshold={threshold:.2f}) ===")
    header = (
        f"{'NAME':<32}  {'SCORE':>7}  {'PCT':>5}  " f"{'PREDICTED':<11}  {'EXPECTED':<20}  PASS"
    )
    print(header)
    print("-" * len(header))
    for r in results:
        expected = f"{r.min_band}..{r.max_band}"
        pct = f"{r.percentile:5.1f}" if r.percentile is not None else "  n/a"
        icon = "OK" if r.passed else "FAIL"
        print(
            f"{r.name:<32}  {r.score:7.4f}  {pct}  "
            f"{r.predicted_band:<11}  {expected:<20}  {icon}"
        )
    print()
    passed = sum(1 for r in results if r.passed)
    total = len(results)
    print(f"{passed}/{total} fixtures in expected band")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Benchmark argot model against a fixture corpus of known paradigm breaks"
    )
    parser.add_argument("--model", default=".argot/model.pkl")
    parser.add_argument(
        "--dataset",
        default=".argot/dataset.jsonl",
        help="Dataset JSONL used to compute percentile context; omitted if missing",
    )
    parser.add_argument("--fixtures", default=str(DEFAULT_FIXTURES_DIR))
    parser.add_argument("--threshold", type=float, default=0.5)
    args = parser.parse_args()

    model_path = Path(args.model)
    if not model_path.exists():
        print(f"error: model not found at {model_path}", file=sys.stderr)
        sys.exit(2)

    fixtures_dir = Path(args.fixtures)
    if not (fixtures_dir / "manifest.json").exists():
        print(f"error: manifest.json not found in {fixtures_dir}", file=sys.stderr)
        sys.exit(2)

    bundle = joblib.load(model_path)
    vectorizer = bundle["vectorizer"]
    input_dim: int = bundle["input_dim"]
    embed_dim: int = bundle["embed_dim"]

    encoder = TokenEncoder(input_dim, embed_dim)
    encoder.load_state_dict(bundle["encoder_state"])
    predictor = ArgotPredictor(embed_dim=embed_dim)
    predictor.load_state_dict(bundle["predictor_state"])
    model = JEPAArgot(encoder, predictor)
    model.eval()

    specs = load_manifest(fixtures_dir)
    distribution = _build_distribution(Path(args.dataset), vectorizer, model)

    def score_fn(spec: FixtureSpec, fixtures_dir: Path) -> float | None:
        return _score_fixture_impl(spec, fixtures_dir, vectorizer, model)

    results = run_benchmark(specs, fixtures_dir, score_fn, args.threshold, distribution)
    _print_report(results, args.threshold)
    sys.exit(0 if all(r.passed for r in results) else 1)


if __name__ == "__main__":
    main()
