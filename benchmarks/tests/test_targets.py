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


def test_python_targets_have_single_head_sha():
    path = Path(__file__).parent.parent / "targets.yaml"
    by_name = {t.name: t for t in load_targets(path)}
    for name in ("fastapi", "rich", "faker"):
        assert len(by_name[name].prs) == 1
        assert by_name[name].prs[0].pr == 0


def test_target_record_fields():
    t = Target(
        name="demo",
        url="https://example.com/demo",
        language="python",
        prs=[],
    )
    assert t.name == "demo"
    assert t.language == "python"
