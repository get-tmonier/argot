from __future__ import annotations

from argot.explain import percentile_rank, select_style_examples


def _make_scored(n: int, base_score: float = 0.1, repo: str = "home") -> list[dict]:  # type: ignore[type-arg]
    return [
        {
            "hunk_tokens": [{"text": f"hunk{i}", "node_type": "identifier"}],
            "context_before": [{"text": f"ctx{i}", "node_type": "identifier"}],
            "file_path": f"file{i % 5}.py",
            "_score": base_score + i * 0.01,
            "_repo": repo,
        }
        for i in range(n)
    ]


def test_percentile_rank_min() -> None:
    # nothing below the minimum → 0%
    assert percentile_rank(0.1, [0.1, 0.5, 0.9]) == 0


def test_percentile_rank_max() -> None:
    # 2 out of 3 values are below 0.9 → ~67%
    rank = percentile_rank(0.9, [0.1, 0.5, 0.9])
    assert 60 < rank < 75


def test_percentile_rank_middle() -> None:
    # 1 out of 3 values is below 0.5 → ~33%
    rank = percentile_rank(0.5, [0.1, 0.5, 0.9])
    assert 25 < rank < 45


def test_select_style_examples_picks_lowest_surprise() -> None:
    records = _make_scored(20)
    examples = select_style_examples(records, n=5)
    scores = [r["_score"] for r in examples]
    assert scores == sorted(scores)
    assert len(examples) == 5


def test_select_style_examples_diverse_files() -> None:
    records = _make_scored(20)
    examples = select_style_examples(records, n=5)
    file_paths = [r["file_path"] for r in examples]
    assert len(set(file_paths)) > 1


def test_select_style_examples_fewer_than_n() -> None:
    records = _make_scored(3)
    examples = select_style_examples(records, n=5)
    assert len(examples) == 3
