from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from argot.research.signal.context_variants import build_context
from argot.tokenize import language_for_path, tokenize_lines
from argot.train import ModelBundle, train_model
from argot.validate import score_records, split_by_time

CATALOG_DIR = Path(__file__).parent / "catalog"
GATE_DELTA = 0.20
EPOCHS = 20


@dataclass
class ScopeConfig:
    name: str
    path_prefix: str
    paradigm: str


@dataclass
class FixtureSpec:
    name: str
    scope: str
    file: str
    hunk_start_line: int
    hunk_end_line: int
    is_break: bool
    rationale: str
    category: str = "legacy"
    set: str = "v1"


@dataclass
class ScopeResult:
    name: str
    break_mean: float
    ctrl_mean: float
    delta: float
    passed: bool


@dataclass
class EntryResult:
    entry: str
    scope_results: list[ScopeResult]
    fixture_scores: list[dict[str, Any]]
    passed: bool


def load_scopes(entry_dir: Path) -> list[ScopeConfig]:
    data = json.loads((entry_dir / "scopes.json").read_text())
    return [ScopeConfig(**s) for s in data["scopes"]]


def load_manifest(entry_dir: Path) -> list[FixtureSpec]:
    data = json.loads((entry_dir / "manifest.json").read_text())
    return [FixtureSpec(**f) for f in data["fixtures"]]


def load_corpus(entry_dir: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    with (entry_dir / "corpus.jsonl").open() as f:
        for line in f:
            if line.strip():
                records.append(json.loads(line))
    return records


def fixture_to_record(
    entry_dir: Path, spec: FixtureSpec, context_mode: str = "file_only"
) -> dict[str, Any]:
    fixture_path = entry_dir / spec.file
    source = fixture_path.read_text(encoding="utf-8")
    lines = source.splitlines()
    lang = language_for_path(str(fixture_path)) or "python"
    hunk_start = spec.hunk_start_line - 1
    hunk_end = spec.hunk_end_line
    ctx_fallback = False
    ctx_truncated = False
    if context_mode != "baseline":
        result = build_context(source, spec.hunk_start_line, spec.hunk_end_line, context_mode)
        ctx_tokens_dicts = result.tokens
        ctx_fallback = result.variant_fallback
        ctx_truncated = result.truncated
    else:
        ctx_start = max(0, hunk_start - 20)
        ctx_tokens_dicts = [
            {"text": t.text} for t in tokenize_lines(lines, lang, ctx_start, hunk_start)
        ]
    hunk_tokens = tokenize_lines(lines, lang, hunk_start, hunk_end)
    hunk_source_lines = lines[hunk_start:hunk_end]
    return {
        "_repo": "acceptance-fixture",
        "author_date_iso": "0",  # fixture records are never time-split; sentinel value
        "language": lang,
        "context_before": ctx_tokens_dicts,
        "context_after": [],
        "hunk_tokens": [
            {"text": t.text, "node_type": t.node_type, "start_line": t.start_line}
            for t in hunk_tokens
        ],
        "hunk_source": "\n".join(hunk_source_lines),
        "hunk_start_line": spec.hunk_start_line,
        "hunk_end_line": spec.hunk_end_line,
        "_ctx_fallback": ctx_fallback,
        "_ctx_truncated": ctx_truncated,
        "_fixture_path": str(fixture_path.resolve()),
    }


def run_entry(entry_dir: Path, epochs: int = EPOCHS) -> EntryResult:
    entry_name = entry_dir.name
    scopes = load_scopes(entry_dir)
    corpus = load_corpus(entry_dir)
    fixtures = load_manifest(entry_dir)

    bundles: dict[str, ModelBundle] = {}
    for scope in scopes:
        scope_records = [r for r in corpus if r.get("file_path", "").startswith(scope.path_prefix)]
        if len(scope_records) < 10:
            raise RuntimeError(
                f"Entry {entry_name!r}, scope {scope.name!r}: "
                f"only {len(scope_records)} records (need ≥ 10)"
            )
        train_records, _ = split_by_time(scope_records, ratio=0.8)
        print(
            f"  [{scope.name}] {len(train_records)} train records, " f"training {epochs} epochs...",
            flush=True,
        )
        bundles[scope.name] = train_model(train_records, encoder="pretrained", epochs=epochs)

    fixture_scores: list[dict[str, Any]] = []
    for spec in fixtures:
        record = fixture_to_record(entry_dir, spec)
        scores = score_records(bundles[spec.scope], [record])
        score = scores[0] if scores else 0.0
        tag = "BREAK" if spec.is_break else "CTRL "
        print(
            f"  [{tag}][{spec.scope}] {spec.name:<40s} score={score:.4f}",
            flush=True,
        )
        fixture_scores.append(
            {
                "name": spec.name,
                "scope": spec.scope,
                "score": score,
                "is_break": spec.is_break,
            }
        )

    scope_results: list[ScopeResult] = []
    for scope in scopes:
        scope_fs = [f for f in fixture_scores if f["scope"] == scope.name]
        breaks = [f["score"] for f in scope_fs if f["is_break"]]
        controls = [f["score"] for f in scope_fs if not f["is_break"]]
        if not breaks:
            raise RuntimeError(f"Scope {scope.name!r}: no break fixtures in manifest")
        if not controls:
            raise RuntimeError(f"Scope {scope.name!r}: no control fixtures in manifest")
        bm = sum(breaks) / len(breaks) if breaks else 0.0
        cm = sum(controls) / len(controls) if controls else 0.0
        delta = bm - cm
        scope_results.append(
            ScopeResult(
                name=scope.name,
                break_mean=bm,
                ctrl_mean=cm,
                delta=delta,
                passed=delta >= GATE_DELTA,
            )
        )

    return EntryResult(
        entry=entry_name,
        scope_results=scope_results,
        fixture_scores=fixture_scores,
        passed=all(s.passed for s in scope_results),
    )


def _write_markdown(result: EntryResult, output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w") as f:
        f.write(f"# Acceptance Test: {result.entry}\n\n")
        f.write("| fixture | scope | score | type |\n|---|---|---|---|\n")
        for fs in result.fixture_scores:
            t = "break" if fs["is_break"] else "control"
            f.write(f"| {fs['name']} | {fs['scope']} | {fs['score']:.4f} | {t} |\n")
        f.write("\n")
        for sr in result.scope_results:
            gate = "GO ✓" if sr.passed else "NO-GO ✗"
            f.write(
                f"**[{sr.name}]** control={sr.ctrl_mean:.4f}  "
                f"break={sr.break_mean:.4f}  delta={sr.delta:.4f}  {gate}\n"
            )
        overall = "GO ✓" if result.passed else "NO-GO ✗"
        f.write(f"\n**Overall:** {overall}\n")


def main() -> None:
    parser = argparse.ArgumentParser(description="Run acceptance tests on catalog entries")
    parser.add_argument("--entry", help="Run a single catalog entry by name")
    parser.add_argument(
        "--catalog",
        default=str(CATALOG_DIR),
        help="Path to catalog directory (default: acceptance/catalog/)",
    )
    parser.add_argument(
        "--out",
        default="docs/research/scoring/acceptance",
        help="Directory to write results markdown",
    )
    parser.add_argument("--epochs", type=int, default=EPOCHS)
    args = parser.parse_args()

    catalog = Path(args.catalog)
    out_dir = Path(args.out)

    entries = (
        [catalog / args.entry] if args.entry else sorted(e for e in catalog.iterdir() if e.is_dir())
    )

    if not entries:
        print("No catalog entries found.", file=sys.stderr)
        sys.exit(1)

    all_passed = True
    for entry_dir in entries:
        print(f"\n=== {entry_dir.name} ===", flush=True)
        try:
            result = run_entry(entry_dir, epochs=args.epochs)
        except RuntimeError as e:
            print(f"  ERROR: {e}", file=sys.stderr)
            all_passed = False
            continue

        for sr in result.scope_results:
            gate = "GO ✓" if sr.passed else "NO-GO ✗"
            print(
                f"  [{sr.name}] control={sr.ctrl_mean:.4f}  "
                f"break={sr.break_mean:.4f}  delta={sr.delta:.4f}  {gate}"
            )
        _write_markdown(result, out_dir / f"{entry_dir.name}.md")
        if not result.passed:
            all_passed = False

    sys.exit(0 if all_passed else 1)


if __name__ == "__main__":
    main()
