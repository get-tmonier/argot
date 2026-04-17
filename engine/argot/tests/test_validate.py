from __future__ import annotations

from argot.validate import (
    compute_auc,
    compute_percentiles,
    inject_foreign,
    shuffle_negatives,
    split_by_time,
)


def _make_records(n: int, base_ts: int = 1_700_000_000, repo: str = "home") -> list[dict]:  # type: ignore[type-arg]
    return [
        {
            "commit_sha": f"sha{i:04d}",
            "author_date_iso": str(base_ts + i * 3600),
            "_repo": repo,
            "context_before": [
                {"text": f"ctx{i}", "node_type": "identifier", "start_line": 0, "end_line": 1}
            ],
            "hunk_tokens": [
                {"text": f"hunk{i}", "node_type": "identifier", "start_line": 1, "end_line": 2}
            ],
            "context_after": [],
            "file_path": "foo.py",
            "language": "python",
            "hunk_start_line": 1,
            "hunk_end_line": 2,
            "parent_sha": None,
        }
        for i in range(n)
    ]


def test_split_by_time_80_20() -> None:
    records = _make_records(100)
    train, held_out = split_by_time(records, ratio=0.8)
    assert len(train) == 80
    assert len(held_out) == 20
    assert max(int(r["author_date_iso"]) for r in train) < min(
        int(r["author_date_iso"]) for r in held_out
    )


def test_shuffle_negatives_same_length() -> None:
    records = _make_records(20)
    shuffled = shuffle_negatives(records, seed=42)
    assert len(shuffled) == len(records)
    orig_pairs = [(r["context_before"][0]["text"], r["hunk_tokens"][0]["text"]) for r in records]
    shuf_pairs = [(r["context_before"][0]["text"], r["hunk_tokens"][0]["text"]) for r in shuffled]
    assert orig_pairs != shuf_pairs


def test_inject_foreign_replaces_hunks() -> None:
    home = _make_records(10, repo="home")
    foreign = _make_records(5, base_ts=1_800_000_000, repo="foreign")
    injected = inject_foreign(home, foreign, seed=0)
    assert len(injected) == len(home)
    # context_before is preserved, hunk_tokens come from foreign pool
    foreign_hunks = {r["hunk_tokens"][0]["text"] for r in foreign}
    for r in injected:
        assert r["context_before"] == home[injected.index(r)]["context_before"]
        assert r["hunk_tokens"][0]["text"] in foreign_hunks


def test_compute_percentiles() -> None:
    scores = [float(i) for i in range(101)]  # 0..100
    p = compute_percentiles(scores)
    assert p["min"] == 0.0
    assert p["p25"] == 25.0
    assert p["median"] == 50.0
    assert p["p75"] == 75.0
    assert p["p95"] == 95.0
    assert p["max"] == 100.0


def test_compute_auc_perfect_separation() -> None:
    good = [0.1, 0.2, 0.15]
    bad = [0.9, 0.8, 0.95]
    auc = compute_auc(good, bad)
    assert auc == 1.0


def test_compute_auc_no_separation() -> None:
    scores = [0.5, 0.5, 0.5]
    auc = compute_auc(scores, scores)
    assert auc == 0.5
