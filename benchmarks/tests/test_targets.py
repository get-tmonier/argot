from pathlib import Path

from argot_bench.targets import Target, load_targets


def test_load_targets_finds_six_corpora():
    path = Path(__file__).parent.parent / "targets.yaml"
    targets = load_targets(path)
    names = sorted(t.name for t in targets)
    assert names == ["faker", "faker-js", "fastapi", "hono", "ink", "rich"]


def test_every_target_has_at_least_one_pr():
    path = Path(__file__).parent.parent / "targets.yaml"
    for t in load_targets(path):
        assert len(t.prs) >= 1, f"{t.name} has no PRs pinned"
        for pr in t.prs:
            assert len(pr.sha) == 40, f"{t.name} PR {pr.pr} SHA not full-length"


def test_all_corpora_have_five_prs():
    """Gate 6: every corpus has exactly 5 PR entries."""
    from pathlib import Path

    from argot_bench.targets import load_targets

    targets_yaml = Path(__file__).parent.parent / "targets.yaml"
    targets = load_targets(targets_yaml)
    for t in targets:
        pr_entries = [p for p in t.prs if p.pr != 0]
        assert len(pr_entries) == 5, (
            f"{t.name} has {len(pr_entries)} PRs, expected 5"
        )


def test_target_record_fields():
    t = Target(
        name="demo",
        url="https://example.com/demo",
        language="python",
        prs=[],
    )
    assert t.name == "demo"
    assert t.language == "python"
