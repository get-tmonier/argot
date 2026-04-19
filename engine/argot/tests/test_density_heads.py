from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import pytest

from argot.corpus import run_benchmark_density
from argot.jepa.density_heads import GmmHead, KnnHead, make_head


def _make_tagged_dataset(path: Path, n_per_repo: int = 40) -> None:
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


def test_knn_head_fit_score() -> None:
    head = KnnHead(k=3)
    rng = np.random.default_rng(0)
    train = rng.standard_normal((50, 8)).astype(np.float32)
    head.fit(train)
    scores = head.score(train[:5])
    assert scores.shape == (5,)
    assert np.all(scores >= 0)


def test_gmm_head_fit_score() -> None:
    head = GmmHead(n_components=2, seed=0)
    rng = np.random.default_rng(1)
    train = rng.standard_normal((60, 8)).astype(np.float32)
    head.fit(train)
    scores = head.score(train[:5])
    assert scores.shape == (5,)


def test_make_head_kinds() -> None:
    for kind in ("knn-20", "gmm-8", "gmm-16", "gmm-32"):
        head = make_head(kind, seed=0)  # type: ignore[arg-type]
        assert head is not None


@pytest.mark.parametrize("head_kind", ["knn-20", "gmm-8"])
def test_run_benchmark_density_writes_rows(tmp_path: Path, head_kind: str) -> None:
    dataset = tmp_path / "combined.jsonl"
    out = tmp_path / f"results-{head_kind}.jsonl"
    _make_tagged_dataset(dataset, n_per_repo=40)

    run_benchmark_density(
        dataset=dataset,
        sizes=[40],
        seeds=2,
        output=out,
        batch_size=16,
        head_kind=head_kind,  # type: ignore[arg-type]
        epochs=1,
    )

    rows = [json.loads(line) for line in out.read_text().strip().splitlines()]
    assert len(rows) == 2  # 1 size × 2 seeds
    for row in rows:
        assert row["head"] == head_kind
        assert row["encoder"] == "bpe"
        assert "synthetic_auc_mean" in row
        assert "shuffled_auc" in row
        for name in ("case_swap", "debug_inject", "error_flip", "quote_flip"):
            assert f"synthetic_auc_{name}" in row
