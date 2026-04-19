#!/usr/bin/env python3
"""Phase 8 spot-check: train pretrained encoder on argot CLI, score all fixtures."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

# Add engine to path when run directly
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from argot.benchmark import DEFAULT_FIXTURES_DIR, load_manifest
from argot.tokenize import language_for_path, tokenize_lines
from argot.train import train_model
from argot.validate import score_records, split_by_time

FIXTURES_DIR = DEFAULT_FIXTURES_DIR
DATASET_PATH = Path(".argot/dataset.jsonl")
OUTPUT_PATH = Path("docs/research/scoring/phase-8/spot-check.md")


def load_jsonl(path: Path, max_records: int = 2000) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    with path.open() as f:
        for line in f:
            if not line.strip():
                continue
            r = json.loads(line)
            records.append(
                {
                    "_repo": r.get("_repo", "argot"),
                    "author_date_iso": r["author_date_iso"],
                    "language": r["language"],
                    "context_before": [{"text": t["text"]} for t in r["context_before"]],
                    "context_after": [{"text": t["text"]} for t in r.get("context_after", [])],
                    "hunk_tokens": [{"text": t["text"]} for t in r["hunk_tokens"]],
                }
            )
            if len(records) >= max_records:
                break
    return records


def fixture_to_record(fixture_path: Path, start_line: int, end_line: int) -> dict[str, Any]:
    """Convert a fixture file hunk into a record the model can score."""
    source = fixture_path.read_text(encoding="utf-8")
    lines = source.splitlines()

    # context_before: lines before the hunk (up to 20 lines), 0-indexed
    ctx_start = max(0, start_line - 21)
    # hunk: lines [start_line-1, end_line-1) in 0-indexed terms
    hunk_start_idx = start_line - 1
    hunk_end_idx = end_line - 1

    lang = language_for_path(str(fixture_path))
    if lang is None:
        # Fall back to python for unrecognised extensions
        lang = "python"

    # tokenize_lines takes (source_lines, lang, start_line, end_line) — 0-indexed, end exclusive
    ctx_tokens = tokenize_lines(lines, lang, ctx_start, hunk_start_idx)
    hunk_tokens = tokenize_lines(lines, lang, hunk_start_idx, hunk_end_idx)

    return {
        "_repo": "argot-cli",
        "author_date_iso": "0",
        "language": lang,
        "context_before": [{"text": t.text} for t in ctx_tokens],
        "context_after": [],
        "hunk_tokens": [{"text": t.text} for t in hunk_tokens],
    }


def main() -> None:
    if not DATASET_PATH.exists():
        print(f"ERROR: {DATASET_PATH} not found. Run `just extract .` first.", file=sys.stderr)
        sys.exit(1)

    print("Loading corpus...", flush=True)
    records = load_jsonl(DATASET_PATH, max_records=2000)
    print(f"  {len(records)} records loaded", flush=True)

    train_records, held_out = split_by_time(records, ratio=0.8)
    print(f"  train={len(train_records)}, held_out={len(held_out)}", flush=True)

    print("Training pretrained encoder (epochs=5)...", flush=True)
    bundle = train_model(train_records, encoder="pretrained", epochs=5)
    print("  done", flush=True)

    print("Scoring held-out (baseline)...", flush=True)
    good_scores = score_records(bundle, held_out)
    good_mean = sum(good_scores) / len(good_scores) if good_scores else 0.0
    print(f"  held-out mean score: {good_mean:.4f}", flush=True)

    print("Loading fixtures...", flush=True)
    specs = load_manifest(FIXTURES_DIR)

    results: list[dict[str, Any]] = []
    for spec in specs:
        fixture_path = FIXTURES_DIR / spec.file
        record = fixture_to_record(fixture_path, spec.hunk_start_line, spec.hunk_end_line)
        scores = score_records(bundle, [record])
        score = scores[0] if scores else 0.0
        is_break = spec.name.startswith("paradigm_break")
        results.append(
            {
                "name": spec.name,
                "score": score,
                "is_break": is_break,
                "rationale": spec.rationale,
            }
        )
        tag = "BREAK" if is_break else "CTRL "
        print(f"  [{tag}] {spec.name:<40s} score={score:.4f}", flush=True)

    break_scores = [r["score"] for r in results if r["is_break"]]
    ctrl_scores = [r["score"] for r in results if not r["is_break"]]
    break_mean = sum(break_scores) / len(break_scores) if break_scores else 0.0
    ctrl_mean = sum(ctrl_scores) / len(ctrl_scores) if ctrl_scores else 0.0
    delta = break_mean - ctrl_mean

    print("\n--- RESULTS ---")
    print(f"  control mean:       {ctrl_mean:.4f}")
    print(f"  paradigm_break mean:{break_mean:.4f}")
    print(f"  delta:              {delta:.4f}  (gate: ≥ 0.20)")
    gate = "GO ✓" if delta >= 0.20 else "NO-GO ✗"
    print(f"  GATE: {gate}")

    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with OUTPUT_PATH.open("w") as f:
        f.write("# Phase 8 Spot-Check Results\n\n")
        f.write("**Gate criterion:** delta ≥ 0.20\n\n")
        f.write("| fixture | score | type |\n|---|---|---|\n")
        for r in results:
            t = "break" if r["is_break"] else "control"
            f.write(f"| {r['name']} | {r['score']:.4f} | {t} |\n")
        f.write(f"\n**Control mean:** {ctrl_mean:.4f}\n")
        f.write(f"**Paradigm-break mean:** {break_mean:.4f}\n")
        f.write(f"**Delta:** {delta:.4f}\n")
        f.write(f"**Gate:** {'GO ✓' if delta >= 0.20 else 'NO-GO ✗'}\n")

    print(f"\nResults written to {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
