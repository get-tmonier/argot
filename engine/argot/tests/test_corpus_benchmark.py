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
