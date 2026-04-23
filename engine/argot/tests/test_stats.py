from __future__ import annotations

from argot.stats import compute_auc, compute_percentiles, split_by_time


def _make_records(n: int, base_ts: int = 1_700_000_000) -> list[dict]:  # type: ignore[type-arg]
    return [{"author_date_iso": str(base_ts + i * 3600), "data": i} for i in range(n)]


def test_split_by_time_ratio() -> None:
    records = _make_records(100)
    train, held_out = split_by_time(records, ratio=0.8)
    assert len(train) == 80
    assert len(held_out) == 20


def test_split_by_time_temporal_order() -> None:
    records = _make_records(10)
    train, held_out = split_by_time(records, ratio=0.8)
    assert max(int(r["author_date_iso"]) for r in train) < min(
        int(r["author_date_iso"]) for r in held_out
    )


def test_split_by_time_deterministic() -> None:
    records = _make_records(20)
    a, b = split_by_time(records, ratio=0.7)
    c, d = split_by_time(records, ratio=0.7)
    assert [r["data"] for r in a] == [r["data"] for r in c]
    assert [r["data"] for r in b] == [r["data"] for r in d]


def test_compute_percentiles_shape() -> None:
    scores = [float(i) for i in range(101)]
    p = compute_percentiles(scores)
    assert set(p.keys()) == {"min", "p25", "median", "p75", "p95", "max"}


def test_compute_percentiles_values() -> None:
    scores = [float(i) for i in range(101)]
    p = compute_percentiles(scores)
    assert p["min"] == 0.0
    assert p["p25"] == 25.0
    assert p["median"] == 50.0
    assert p["max"] == 100.0


def test_compute_auc_perfect_separation() -> None:
    good = [0.1, 0.2, 0.15]
    bad = [0.9, 0.8, 0.95]
    assert compute_auc(good, bad) == 1.0


def test_compute_auc_no_separation() -> None:
    scores = [0.5, 0.5, 0.5]
    assert compute_auc(scores, scores) == 0.5
