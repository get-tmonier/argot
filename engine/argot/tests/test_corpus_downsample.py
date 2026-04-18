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
