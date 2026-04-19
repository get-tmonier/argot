#!/usr/bin/env python3
"""Phase 8 spot-check: two-scope training on argot's own history, one model per package."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from argot.benchmark import DEFAULT_FIXTURES_DIR, load_manifest
from argot.tokenize import language_for_path, tokenize_lines
from argot.train import ModelBundle, train_model
from argot.validate import score_records, split_by_time

FIXTURES_DIR = DEFAULT_FIXTURES_DIR
# Argot's own git history — multi-scope extract covers both cli/ and engine/
CORPUS_PATH = Path(".argot/dataset.jsonl")
OUTPUT_PATH = Path("docs/research/scoring/phase-8/spot-check.md")

TS_LANGS = {"typescript", "javascript"}


def load_jsonl_by_path_prefix(path: Path, path_prefix: str) -> list[dict[str, Any]]:
    """Load records from argot's own corpus filtered to a specific path scope."""
    records: list[dict[str, Any]] = []
    with path.open() as f:
        for line in f:
            if not line.strip():
                continue
            r = json.loads(line)
            if not r.get("file_path", "").startswith(path_prefix):
                continue
            records.append({
                "_repo": r.get("_repo", "argot"),
                "author_date_iso": r["author_date_iso"],
                "language": r["language"],
                "context_before": [{"text": t["text"]} for t in r["context_before"]],
                "context_after": [{"text": t["text"]} for t in r.get("context_after", [])],
                "hunk_tokens": [{"text": t["text"]} for t in r["hunk_tokens"]],
            })
    return records


def fixture_to_record(fixture_path: Path, start_line: int, end_line: int) -> dict[str, Any]:
    source = fixture_path.read_text(encoding="utf-8")
    lines = source.splitlines()
    ctx_start = max(0, start_line - 21)
    hunk_start_idx = start_line - 1
    hunk_end_idx = end_line - 1
    lang = language_for_path(str(fixture_path)) or "python"
    ctx_tokens = tokenize_lines(lines, lang, ctx_start, hunk_start_idx)
    hunk_tokens = tokenize_lines(lines, lang, hunk_start_idx, hunk_end_idx)
    return {
        "_repo": "argot-fixture",
        "author_date_iso": "0",
        "language": lang,
        "context_before": [{"text": t.text} for t in ctx_tokens],
        "context_after": [],
        "hunk_tokens": [{"text": t.text} for t in hunk_tokens],
    }


def train_scope(
    label: str, path_prefix: str, epochs: int = 20
) -> tuple[ModelBundle, list[dict[str, Any]]]:
    print(f"\n=== {label} scope (path: {path_prefix!r}) ===", flush=True)
    if not CORPUS_PATH.exists():
        print(f"  ERROR: {CORPUS_PATH} not found. Run `just extract .` first.", file=sys.stderr)
        sys.exit(1)
    records = load_jsonl_by_path_prefix(CORPUS_PATH, path_prefix)
    print(f"  {len(records)} records from argot history under {path_prefix!r}", flush=True)
    if len(records) < 10:
        print(f"  ERROR: too few records ({len(records)}) — try `just extract .` to refresh", file=sys.stderr)
        sys.exit(1)
    train_records, held_out = split_by_time(records, ratio=0.8)
    print(f"  train={len(train_records)}, held_out={len(held_out)}", flush=True)
    print(f"  Training pretrained encoder (epochs={epochs})...", flush=True)
    bundle = train_model(train_records, encoder="pretrained", epochs=epochs)
    good_scores = score_records(bundle, held_out)
    good_mean = sum(good_scores) / len(good_scores) if good_scores else 0.0
    print(f"  held-out mean: {good_mean:.4f}", flush=True)
    return bundle, held_out


def main() -> None:
    # cli/ → TypeScript/Effect model (argot CLI package)
    # engine/ → Python model (argot engine package)
    ts_bundle, _ = train_scope("cli (TypeScript)", "cli/", epochs=20)
    py_bundle, _ = train_scope("engine (Python)", "engine/", epochs=20)

    print("\n=== Scoring fixtures ===", flush=True)
    specs = load_manifest(FIXTURES_DIR)

    results: list[dict[str, Any]] = []
    for spec in specs:
        fixture_path = FIXTURES_DIR / spec.file
        record = fixture_to_record(fixture_path, spec.hunk_start_line, spec.hunk_end_line)
        lang = record["language"]
        bundle = ts_bundle if lang in TS_LANGS else py_bundle
        scope = "cli" if lang in TS_LANGS else "engine"
        scores = score_records(bundle, [record])
        score = scores[0] if scores else 0.0
        is_break = spec.name.startswith("paradigm_break")
        results.append({"name": spec.name, "score": score, "is_break": is_break, "scope": scope})
        tag = "BREAK" if is_break else "CTRL "
        print(f"  [{tag}][{scope}] {spec.name:<40s} score={score:.4f}", flush=True)

    def stats(subset: list[dict[str, Any]]) -> tuple[float, float, float]:
        breaks = [r["score"] for r in subset if r["is_break"]]
        ctrls = [r["score"] for r in subset if not r["is_break"]]
        bm = sum(breaks) / len(breaks) if breaks else 0.0
        cm = sum(ctrls) / len(ctrls) if ctrls else 0.0
        return bm, cm, bm - cm

    cli_results = [r for r in results if r["scope"] == "cli"]
    eng_results = [r for r in results if r["scope"] == "engine"]
    cli_bm, cli_cm, cli_delta = stats(cli_results)
    eng_bm, eng_cm, eng_delta = stats(eng_results)
    all_bm, all_cm, all_delta = stats(results)

    print("\n--- RESULTS ---")
    print(f"  [cli]    control={cli_cm:.4f}  break={cli_bm:.4f}  delta={cli_delta:.4f}")
    print(f"  [engine] control={eng_cm:.4f}  break={eng_bm:.4f}  delta={eng_delta:.4f}")
    print(f"  [all]    control={all_cm:.4f}  break={all_bm:.4f}  delta={all_delta:.4f}  (gate: ≥ 0.20)")
    gate = "GO ✓" if all_delta >= 0.20 else "NO-GO ✗"
    print(f"  GATE: {gate}")

    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with OUTPUT_PATH.open("w") as f:
        f.write("# Phase 8 Spot-Check Results\n\n")
        f.write("**Training:** each scope trains on argot's own git history filtered by path\n")
        f.write("- `cli/` scope → TypeScript/Effect model (argot CLI package)\n")
        f.write("- `engine/` scope → Python model (argot engine package)\n\n")
        f.write("**Gate criterion:** overall delta ≥ 0.20\n\n")
        f.write("| fixture | scope | score | type |\n|---|---|---|---|\n")
        for r in results:
            t = "break" if r["is_break"] else "control"
            f.write(f"| {r['name']} | {r['scope']} | {r['score']:.4f} | {t} |\n")
        f.write(f"\n**CLI (TypeScript):** control={cli_cm:.4f}  break={cli_bm:.4f}  delta={cli_delta:.4f}\n")
        f.write(f"**Engine (Python):** control={eng_cm:.4f}  break={eng_bm:.4f}  delta={eng_delta:.4f}\n")
        f.write(f"**Overall delta:** {all_delta:.4f}\n")
        f.write(f"**Gate:** {'GO ✓' if all_delta >= 0.20 else 'NO-GO ✗'}\n")
    print(f"\nResults written to {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
