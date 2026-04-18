# Scoring Benchmark — Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the benchmark infrastructure so we can run `argot-engine validate` at varying dataset sizes against a mixed-repo corpus and produce a machine-readable `results.jsonl` of AUC numbers.

**Architecture:** One new Python module (`engine/argot/corpus.py`) with `concat` and `benchmark` subcommands. One small modification to `engine/argot/extract.py` to stamp a `_repo` tag onto output records. Everything else composes the existing `validate.py` primitives — no changes to model, training, or scoring.

**Tech Stack:** Python 3.13, argparse subparsers, pytest, uv, just.

**Design doc:** [`docs/research/scoring/DESIGN.md`](../../research/scoring/DESIGN.md) §Phase 1.
**Roadmap:** [`docs/research/scoring/ROADMAP.md`](../../research/scoring/ROADMAP.md) — update when tasks complete.

---

## Task 1: Add `--repo-name` flag to extract, stamp `_repo` on records

**Why:** `validate.py` already activates its cross-repo AUC mechanism when records carry a `_repo` tag (validate.py:145-157), but `extract.py` never writes one. This is the enabling change.

**Files:**
- Modify: `engine/argot/extract.py`
- Modify: `engine/argot/tests/test_extract_smoke.py`

### - [ ] Step 1.1: Add the failing test

Append the following test to `engine/argot/tests/test_extract_smoke.py`:

```python
def test_smoke_extract_with_repo_name(tmp_path: Path) -> None:
    """--repo-name stamps _repo on every output record."""
    out = tmp_path / "dataset.jsonl"
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "argot.extract",
            str(REPO_ROOT),
            "--out",
            str(out),
            "--repo-name",
            "argot",
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode in (0, 2), f"stderr: {result.stderr}"
    if result.returncode == 0:
        lines = out.read_text().strip().splitlines()
        assert len(lines) >= 1
        for line in lines:
            record = json.loads(line)
            assert record.get("_repo") == "argot"


def test_smoke_extract_without_repo_name_omits_tag(tmp_path: Path) -> None:
    """Absence of --repo-name leaves records un-tagged (backwards-compat)."""
    out = tmp_path / "dataset.jsonl"
    result = subprocess.run(
        [sys.executable, "-m", "argot.extract", str(REPO_ROOT), "--out", str(out)],
        capture_output=True,
        text=True,
    )
    assert result.returncode in (0, 2)
    if result.returncode == 0:
        lines = out.read_text().strip().splitlines()
        for line in lines:
            record = json.loads(line)
            assert "_repo" not in record
```

### - [ ] Step 1.2: Run tests to verify they fail

Run: `uv run --project engine --package argot-engine pytest engine/argot/tests/test_extract_smoke.py -v`

Expected: `test_smoke_extract_with_repo_name` FAILS because `--repo-name` is an unknown argument.

### - [ ] Step 1.3: Add the flag and stamp records

Modify `engine/argot/extract.py` in two places.

First, add the argument (after the existing `--limit` argument at line 36):

```python
parser.add_argument(
    "--repo-name",
    default=None,
    help="Tag each record with _repo=<name> for cross-repo AUC (validate.py)",
)
```

Second, replace the record write block (currently lines 104-105):

```python
fh.write(json.dumps(asdict(record)))
fh.write("\n")
```

with:

```python
record_dict = asdict(record)
if args.repo_name is not None:
    record_dict["_repo"] = args.repo_name
fh.write(json.dumps(record_dict))
fh.write("\n")
```

### - [ ] Step 1.4: Run tests to verify they pass

Run: `uv run --project engine --package argot-engine pytest engine/argot/tests/test_extract_smoke.py -v`

Expected: all three tests PASS.

### - [ ] Step 1.5: Run full verify

Run: `just verify`

Expected: all checks pass.

### - [ ] Step 1.6: Commit

```bash
git add engine/argot/extract.py engine/argot/tests/test_extract_smoke.py
git commit -m "feat(extract): add --repo-name flag to stamp _repo on records"
```

---

## Task 2: Create `corpus concat` subcommand

**Why:** We need one combined dataset file from many per-repo extractions before we can run cross-repo benchmarks. Concat also validates that every record is tagged (guardrail against silently producing untagged combined datasets).

**Files:**
- Create: `engine/argot/corpus.py`
- Create: `engine/argot/tests/test_corpus_concat.py`

### - [ ] Step 2.1: Write the failing test

Create `engine/argot/tests/test_corpus_concat.py`:

```python
from __future__ import annotations

import json
from pathlib import Path

import pytest

from argot.corpus import concat_datasets


def _write_jsonl(path: Path, records: list[dict]) -> None:  # type: ignore[type-arg]
    path.write_text("\n".join(json.dumps(r) for r in records) + "\n")


def test_concat_two_tagged_datasets_returns_counts(tmp_path: Path) -> None:
    a = tmp_path / "a.jsonl"
    b = tmp_path / "b.jsonl"
    out = tmp_path / "combined.jsonl"
    _write_jsonl(a, [{"_repo": "alpha", "hunk_tokens": []} for _ in range(3)])
    _write_jsonl(b, [{"_repo": "beta", "hunk_tokens": []} for _ in range(2)])

    counts = concat_datasets([a, b], out)

    assert counts == {"alpha": 3, "beta": 2}
    out_lines = out.read_text().strip().splitlines()
    assert len(out_lines) == 5


def test_concat_rejects_record_missing_repo_tag(tmp_path: Path) -> None:
    a = tmp_path / "a.jsonl"
    out = tmp_path / "combined.jsonl"
    _write_jsonl(a, [{"hunk_tokens": []}])  # no _repo tag

    with pytest.raises(ValueError, match="_repo"):
        concat_datasets([a], out)


def test_concat_skips_blank_lines(tmp_path: Path) -> None:
    a = tmp_path / "a.jsonl"
    out = tmp_path / "combined.jsonl"
    a.write_text('{"_repo": "x", "hunk_tokens": []}\n\n\n')

    counts = concat_datasets([a], out)

    assert counts == {"x": 1}
```

### - [ ] Step 2.2: Run test to verify it fails

Run: `uv run --project engine --package argot-engine pytest engine/argot/tests/test_corpus_concat.py -v`

Expected: all three tests FAIL with `ModuleNotFoundError: No module named 'argot.corpus'`.

### - [ ] Step 2.3: Implement the corpus module with concat

Create `engine/argot/corpus.py`:

```python
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def concat_datasets(inputs: list[Path], output: Path) -> dict[str, int]:
    """Concatenate tagged JSONL datasets; return per-repo record counts.

    Every record must carry a `_repo` tag (set by `argot-extract --repo-name`).
    """
    counts: dict[str, int] = {}
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w") as out_fh:
        for src in inputs:
            for line in src.read_text().splitlines():
                if not line.strip():
                    continue
                record = json.loads(line)
                if "_repo" not in record:
                    raise ValueError(
                        f"record in {src} missing _repo tag "
                        f"(re-extract with --repo-name)"
                    )
                counts[record["_repo"]] = counts.get(record["_repo"], 0) + 1
                out_fh.write(line + "\n")
    return counts


def _cmd_concat(args: argparse.Namespace) -> int:
    inputs = [Path(p) for p in args.inputs]
    for p in inputs:
        if not p.exists():
            print(f"error: input not found: {p}", file=sys.stderr)
            return 2
    counts = concat_datasets(inputs, Path(args.out))
    total = sum(counts.values())
    print(f"wrote {total} records to {args.out}")
    for repo, n in sorted(counts.items()):
        print(f"  {repo}: {n}")
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(description="argot corpus utilities")
    sub = parser.add_subparsers(dest="cmd", required=True)

    concat_p = sub.add_parser("concat", help="Concatenate tagged JSONL datasets")
    concat_p.add_argument("inputs", nargs="+", help="Input JSONL paths")
    concat_p.add_argument("-o", "--out", required=True, help="Output JSONL path")
    concat_p.set_defaults(func=_cmd_concat)

    args = parser.parse_args()
    sys.exit(args.func(args))


if __name__ == "__main__":
    main()
```

### - [ ] Step 2.4: Run tests to verify they pass

Run: `uv run --project engine --package argot-engine pytest engine/argot/tests/test_corpus_concat.py -v`

Expected: all three tests PASS.

### - [ ] Step 2.5: Commit

```bash
git add engine/argot/corpus.py engine/argot/tests/test_corpus_concat.py
git commit -m "feat(corpus): concat subcommand for tagged JSONL datasets"
```

---

## Task 3: Add stratified downsample helper

**Why:** The benchmark needs to run at multiple target sizes (e.g., 500, 2000, 8000) on the same combined dataset. Downsampling must be deterministic (seedable for reproducibility) and stratified (preserve per-repo proportions so cross-repo AUC stays meaningful).

**Files:**
- Modify: `engine/argot/corpus.py`
- Create: `engine/argot/tests/test_corpus_downsample.py`

### - [ ] Step 3.1: Write the failing test

Create `engine/argot/tests/test_corpus_downsample.py`:

```python
from __future__ import annotations

from argot.corpus import stratified_downsample


def _records(n: int, repo: str) -> list[dict]:  # type: ignore[type-arg]
    return [{"_repo": repo, "i": i} for i in range(n)]


def test_downsample_deterministic_with_seed() -> None:
    records = _records(100, "alpha") + _records(100, "beta")
    a = stratified_downsample(records, target_size=50, seed=7)
    b = stratified_downsample(records, target_size=50, seed=7)
    assert [r["i"] for r in a] == [r["i"] for r in b]
    assert [r["_repo"] for r in a] == [r["_repo"] for r in b]


def test_downsample_preserves_repo_proportions() -> None:
    # 3:1 ratio of alpha:beta → target_size 40 → ~30 alpha, ~10 beta
    records = _records(300, "alpha") + _records(100, "beta")
    sample = stratified_downsample(records, target_size=40, seed=0)
    by_repo = {"alpha": 0, "beta": 0}
    for r in sample:
        by_repo[r["_repo"]] += 1
    assert abs(by_repo["alpha"] - 30) <= 1
    assert abs(by_repo["beta"] - 10) <= 1
    assert by_repo["alpha"] + by_repo["beta"] == 40


def test_downsample_target_larger_than_source_returns_all() -> None:
    records = _records(10, "alpha")
    sample = stratified_downsample(records, target_size=100, seed=0)
    assert len(sample) == 10


def test_downsample_different_seeds_produce_different_samples() -> None:
    records = _records(100, "alpha")
    a = stratified_downsample(records, target_size=50, seed=1)
    b = stratified_downsample(records, target_size=50, seed=2)
    assert [r["i"] for r in a] != [r["i"] for r in b]
```

### - [ ] Step 3.2: Run test to verify it fails

Run: `uv run --project engine --package argot-engine pytest engine/argot/tests/test_corpus_downsample.py -v`

Expected: all four tests FAIL with `ImportError: cannot import name 'stratified_downsample' from 'argot.corpus'`.

### - [ ] Step 3.3: Implement the helper

Add to `engine/argot/corpus.py` (near the top, after the imports, before `concat_datasets`):

```python
import random
from typing import Any


def stratified_downsample(
    records: list[dict[str, Any]],
    target_size: int,
    seed: int,
) -> list[dict[str, Any]]:
    """Deterministically sample `target_size` records, preserving per-`_repo` proportions.

    If target_size >= len(records), returns all records. Per-repo picks are floor(target * share)
    with the remainder filled by largest-repo picks to hit target_size exactly when possible.
    """
    if target_size >= len(records):
        return list(records)

    by_repo: dict[str, list[dict[str, Any]]] = {}
    for r in records:
        by_repo.setdefault(r["_repo"], []).append(r)

    total = len(records)
    rng = random.Random(seed)

    sampled: list[dict[str, Any]] = []
    for repo in sorted(by_repo):
        share = round(target_size * len(by_repo[repo]) / total)
        share = min(share, len(by_repo[repo]))
        sampled.extend(rng.sample(by_repo[repo], share))

    return sampled
```

Note: the existing `import random` at the top of the file will absorb this line — if `random` isn't already imported, add it to the top-level imports block instead of inline.

### - [ ] Step 3.4: Run tests to verify they pass

Run: `uv run --project engine --package argot-engine pytest engine/argot/tests/test_corpus_downsample.py -v`

Expected: all four tests PASS.

### - [ ] Step 3.5: Commit

```bash
git add engine/argot/corpus.py engine/argot/tests/test_corpus_downsample.py
git commit -m "feat(corpus): stratified_downsample helper"
```

---

## Task 4: Add `corpus benchmark` subcommand

**Why:** This is the main deliverable — one command that loops over sizes × seeds and writes a `results.jsonl` summarizing AUC at each size.

**Files:**
- Modify: `engine/argot/corpus.py`
- Create: `engine/argot/tests/test_corpus_benchmark.py`

### - [ ] Step 4.1: Write the failing test

Create `engine/argot/tests/test_corpus_benchmark.py`:

```python
from __future__ import annotations

import json
from pathlib import Path

from argot.corpus import run_benchmark


def _make_tagged_dataset(path: Path, n_per_repo: int = 40) -> None:
    """Write a tiny two-repo dataset that validate.py can process end-to-end."""
    records = []
    base_ts = 1_700_000_000
    for repo in ("home", "foreign"):
        for i in range(n_per_repo):
            records.append(
                {
                    "_repo": repo,
                    "commit_sha": f"{repo}-{i:04d}",
                    "author_date_iso": str(base_ts + i * 3600),
                    "file_path": f"{repo}/file_{i % 5}.py",
                    "language": "python",
                    "hunk_start_line": 1,
                    "hunk_end_line": 2,
                    "parent_sha": None,
                    "context_before": [
                        {
                            "text": f"{repo}_ctx_{i % 7}",
                            "node_type": "identifier",
                            "start_line": 0,
                            "end_line": 1,
                        }
                    ],
                    "hunk_tokens": [
                        {
                            "text": f"{repo}_hunk_{i % 11}",
                            "node_type": "identifier",
                            "start_line": 1,
                            "end_line": 2,
                        }
                    ],
                    "context_after": [],
                }
            )
    path.write_text("\n".join(json.dumps(r) for r in records) + "\n")


def test_benchmark_writes_one_row_per_size_seed(tmp_path: Path) -> None:
    dataset = tmp_path / "combined.jsonl"
    out = tmp_path / "results.jsonl"
    _make_tagged_dataset(dataset, n_per_repo=40)  # 80 records total

    run_benchmark(
        dataset=dataset,
        sizes=[40, 60],
        seeds=2,
        output=out,
        epochs=1,
        batch_size=16,
    )

    rows = [json.loads(line) for line in out.read_text().strip().splitlines()]
    assert len(rows) == 4  # 2 sizes × 2 seeds

    for row in rows:
        assert row["size"] in (40, 60)
        assert row["seed"] in (0, 1)
        assert "shuffled_auc" in row
        assert "cross_auc" in row
        assert "injected_auc" in row
        assert "good_median" in row
        assert "good_p95" in row
        assert "n_repos" in row
        assert "trained_at" in row


def test_benchmark_appends_to_existing_output(tmp_path: Path) -> None:
    dataset = tmp_path / "combined.jsonl"
    out = tmp_path / "results.jsonl"
    _make_tagged_dataset(dataset, n_per_repo=40)

    out.write_text('{"prior": "run"}\n')  # simulate prior results

    run_benchmark(
        dataset=dataset,
        sizes=[40],
        seeds=1,
        output=out,
        epochs=1,
        batch_size=16,
    )

    rows = [json.loads(line) for line in out.read_text().strip().splitlines()]
    assert len(rows) == 2  # 1 prior + 1 new
    assert rows[0] == {"prior": "run"}
    assert rows[1]["size"] == 40
```

### - [ ] Step 4.2: Run test to verify it fails

Run: `uv run --project engine --package argot-engine pytest engine/argot/tests/test_corpus_benchmark.py -v`

Expected: both tests FAIL with `ImportError: cannot import name 'run_benchmark' from 'argot.corpus'`.

### - [ ] Step 4.3: Implement `run_benchmark` + CLI wiring

Add to `engine/argot/corpus.py` (after `stratified_downsample`):

```python
from datetime import UTC, datetime

import numpy as np

from argot.train import train_model
from argot.validate import (
    compute_auc,
    inject_foreign,
    score_records,
    shuffle_negatives,
    split_by_time,
)


def run_benchmark(
    *,
    dataset: Path,
    sizes: list[int],
    seeds: int,
    output: Path,
    epochs: int = 20,
    batch_size: int = 128,
) -> None:
    """Run validate-style AUC measurement at each (size, seed); append to output JSONL."""
    records = [
        json.loads(line) for line in dataset.read_text().splitlines() if line.strip()
    ]

    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("a") as out_fh:
        for size in sizes:
            for seed in range(seeds):
                row = _benchmark_one(
                    records=records,
                    size=size,
                    seed=seed,
                    epochs=epochs,
                    batch_size=batch_size,
                )
                out_fh.write(json.dumps(row) + "\n")
                out_fh.flush()
                print(
                    f"size={size:>6d} seed={seed}  "
                    f"shuffled={row['shuffled_auc']:.3f}  "
                    f"cross={row['cross_auc']:.3f}  "
                    f"injected={row['injected_auc']:.3f}"
                )


def _benchmark_one(
    *,
    records: list[dict[str, Any]],
    size: int,
    seed: int,
    epochs: int,
    batch_size: int,
) -> dict[str, Any]:
    sample = stratified_downsample(records, target_size=size, seed=seed)

    repo_groups: dict[str, list[dict[str, Any]]] = {}
    for r in sample:
        repo_groups.setdefault(r["_repo"], []).append(r)

    foreign_name = min(repo_groups, key=lambda n: len(repo_groups[n]))
    foreign = repo_groups[foreign_name]
    home = [r for r in sample if r["_repo"] != foreign_name]

    train_records, held_out = split_by_time(home, ratio=0.8)
    bundle = train_model(train_records, epochs=epochs, batch_size=batch_size)

    good = score_records(bundle, held_out)
    shuffled = score_records(bundle, shuffle_negatives(held_out, seed=seed))
    cross = score_records(bundle, foreign)
    injected = score_records(bundle, inject_foreign(held_out, foreign, seed=seed))

    good_arr = np.array(good) if good else np.array([0.0])
    return {
        "size": size,
        "seed": seed,
        "n_repos": len(repo_groups),
        "n_train": len(train_records),
        "n_held_out": len(held_out),
        "n_foreign": len(foreign),
        "shuffled_auc": compute_auc(good, shuffled),
        "cross_auc": compute_auc(good, cross),
        "injected_auc": compute_auc(good, injected),
        "good_median": float(np.median(good_arr)),
        "good_p95": float(np.percentile(good_arr, 95)),
        "trained_at": datetime.now(UTC).isoformat(),
    }


def _cmd_benchmark(args: argparse.Namespace) -> int:
    dataset = Path(args.dataset)
    if not dataset.exists():
        print(f"error: dataset not found: {dataset}", file=sys.stderr)
        return 2
    sizes = [int(s) for s in args.sizes.split(",")]
    run_benchmark(
        dataset=dataset,
        sizes=sizes,
        seeds=args.seeds,
        output=Path(args.out),
        epochs=args.epochs,
        batch_size=args.batch_size,
    )
    return 0
```

Wire the subcommand: extend `main()` in `engine/argot/corpus.py` — after the block that registers `concat_p`, before `args = parser.parse_args()`:

```python
bench_p = sub.add_parser("benchmark", help="Run AUC benchmark across sizes × seeds")
bench_p.add_argument("--dataset", required=True, help="Combined tagged JSONL")
bench_p.add_argument(
    "--sizes",
    default="500,2000,8000",
    help="Comma-separated target dataset sizes",
)
bench_p.add_argument("--seeds", type=int, default=3, help="Runs per size (seeds 0..N-1)")
bench_p.add_argument(
    "--out", default=".argot/research/results.jsonl", help="Append results to this JSONL"
)
bench_p.add_argument("--epochs", type=int, default=20)
bench_p.add_argument("--batch-size", type=int, default=128)
bench_p.set_defaults(func=_cmd_benchmark)
```

### - [ ] Step 4.4: Run tests to verify they pass

Run: `uv run --project engine --package argot-engine pytest engine/argot/tests/test_corpus_benchmark.py -v`

Expected: both tests PASS. Runtime ~30-60s (training is fast on 80 records × 1 epoch).

### - [ ] Step 4.5: Run full test suite

Run: `uv run --project engine --package argot-engine pytest engine -v`

Expected: all tests PASS (existing + new).

### - [ ] Step 4.6: Commit

```bash
git add engine/argot/corpus.py engine/argot/tests/test_corpus_benchmark.py
git commit -m "feat(corpus): benchmark subcommand — AUC across sizes × seeds"
```

---

## Task 5: Wire console script + justfile target

**Why:** Make the new command reachable via `uv run argot-corpus ...` and `just research benchmark ...`.

**Files:**
- Modify: `engine/pyproject.toml`
- Modify: `justfile`

### - [ ] Step 5.1: Add console script

Edit `engine/pyproject.toml`. In the `[project.scripts]` block (currently ending with `argot-benchmark = "argot.benchmark:main"` at line 23), append:

```toml
argot-corpus = "argot.corpus:main"
```

### - [ ] Step 5.2: Add justfile recipes

Edit `justfile`. After the existing `benchmark` recipe (line 34-35), before `# --- individual checks ---`, add:

```make
# --- research ---

research-concat out +inputs:
    uv run --package argot-engine python -m argot.corpus concat {{inputs}} -o {{out}}

research-benchmark dataset=".argot/research/combined.jsonl" sizes="500,2000,8000" seeds="3":
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset {{dataset}} --sizes {{sizes}} --seeds {{seeds}} \
        --out .argot/research/results.jsonl
```

Note on `just` syntax: `+inputs` means "one or more variadic positional
args"; `out` is a required positional. Usage:
`just research-concat combined.jsonl a.jsonl b.jsonl c.jsonl`.

### - [ ] Step 5.3: Verify the console script is registered

Run: `uv sync --project engine && uv run --project engine --package argot-engine argot-corpus --help`

Expected: the argparse help prints with `concat` and `benchmark` subcommands listed.

### - [ ] Step 5.4: Verify the justfile recipes are listed

Run: `just --list | grep research`

Expected output includes:
```
research-concat out +inputs
research-benchmark dataset=".argot/research/combined.jsonl" sizes="500,2000,8000" seeds="3"
```

### - [ ] Step 5.5: Run full verify

Run: `just verify`

Expected: all checks pass. Knip may flag `argot-corpus` if TypeScript side doesn't reference it — acceptable since it's a Python-side console script.

### - [ ] Step 5.6: Commit

```bash
git add engine/pyproject.toml justfile
git commit -m "chore(corpus): wire argot-corpus console script + just research recipes"
```

---

## Task 6: Update roadmap

**Why:** Phase 1 is now done — mark it on the living tracker so the next session picks up at Phase 2.

**Files:**
- Modify: `docs/research/scoring/ROADMAP.md`

### - [ ] Step 6.1: Tick off Phase 1 items and update the session log

In `docs/research/scoring/ROADMAP.md`, under the "Phase 1 — benchmark infrastructure" section, change the six unchecked `- [ ]` items to `- [x]`.

Update the "Current phase" header line to:

```markdown
**Current phase**: Phase 2 — sizing study (not started)
```

Under "Session log", append a new bullet:

```markdown
- **YYYY-MM-DD**: Phase 1 complete. `argot-corpus concat` and
  `argot-corpus benchmark` land, `extract --repo-name` stamps `_repo`.
  `just research-concat` and `just research-benchmark` wired. Next: Phase 2
  corpus kickoff — pin repo URLs + SHAs in `01-corpus.md`.
```

(Use today's date in YYYY-MM-DD form in place of `YYYY-MM-DD`.)

### - [ ] Step 6.2: Commit

```bash
git add docs/research/scoring/ROADMAP.md
git commit -m "docs(research): mark Phase 1 complete, advance to Phase 2"
```

---

## Final verification

### - [ ] Step F.1: Run full verify one more time

Run: `just verify`

Expected: all lint, format, typecheck, boundaries, knip, and test checks pass.

### - [ ] Step F.2: Confirm the branch is clean

Run: `git status`

Expected: `nothing to commit, working tree clean`.

### - [ ] Step F.3: List the commits made

Run: `git log --oneline $(git merge-base HEAD main)..HEAD`

Expected: six commits (one per task) plus the design/roadmap commit from before this plan.

---

## Notes for the implementer

- **TDD discipline:** Each task's first step is a failing test. Do not skip. If you find yourself wanting to "just write the code quickly," stop — the test is how we know the code is correct.
- **DRY:** `run_benchmark` composes `split_by_time`, `shuffle_negatives`, `inject_foreign`, `score_records`, `compute_auc`, and `train_model` from existing modules. Do NOT copy-paste their logic.
- **YAGNI:** The plan covers only the infrastructure needed for Phase 2. Do NOT add flags, features, or refactors that "might be useful later." Phase 2 and Phase 3 can add what they need.
- **Tiny test fixtures:** `test_corpus_benchmark` uses 80 records × 1 epoch so it runs in under a minute. Real benchmarks use 500-32000 records × 20 epochs and take longer.
- **Don't touch model/training/scoring code:** All changes in this plan are orchestration. If you find yourself reaching into `train.py`, `jepa/*`, or `check.py`, you've gone out of scope.
