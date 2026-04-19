from __future__ import annotations

import json
from collections import Counter
from pathlib import Path

from argot.corpus import _load_records, run_benchmark


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


def test_load_records_strips_to_required_fields(tmp_path: Path) -> None:
    dataset = tmp_path / "data.jsonl"
    _make_tagged_dataset(dataset, n_per_repo=5)

    records = _load_records(dataset)

    assert len(records) == 10
    expected_keys = {"_repo", "author_date_iso", "language", "context_before", "hunk_tokens"}
    for r in records:
        assert set(r.keys()) == expected_keys
        assert r["language"] == "python"  # _make_tagged_dataset hard-codes python
        for token in r["context_before"]:
            assert set(token.keys()) == {"text"}
        for token in r["hunk_tokens"]:
            assert set(token.keys()) == {"text"}


def test_load_records_reservoir_caps_count(tmp_path: Path) -> None:
    dataset = tmp_path / "data.jsonl"
    _make_tagged_dataset(dataset, n_per_repo=50)  # 100 records total

    records = _load_records(dataset, max_records=40)

    assert len(records) == 40


def test_load_records_no_cap_returns_all(tmp_path: Path) -> None:
    dataset = tmp_path / "data.jsonl"
    _make_tagged_dataset(dataset, n_per_repo=20)

    assert len(_load_records(dataset)) == 40
    assert len(_load_records(dataset, max_records=None)) == 40


def test_load_records_reservoir_includes_both_repos(tmp_path: Path) -> None:
    dataset = tmp_path / "data.jsonl"
    _make_tagged_dataset(dataset, n_per_repo=200)  # 400 records total

    # Sample 100 — both repos should be represented (probability of missing
    # one entirely is astronomically small at these counts).
    records = _load_records(dataset, max_records=100)

    repos = Counter(r["_repo"] for r in records)
    assert "home" in repos
    assert "foreign" in repos


def test_load_records_reservoir_when_max_exceeds_total(tmp_path: Path) -> None:
    dataset = tmp_path / "data.jsonl"
    _make_tagged_dataset(dataset, n_per_repo=10)  # 20 records

    # max_records > total → return everything, no truncation
    records = _load_records(dataset, max_records=1000)
    assert len(records) == 20


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
