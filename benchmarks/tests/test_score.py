from pathlib import Path


def test_score_result_fields():
    from argot_bench.score import ScoreResult

    r = ScoreResult(
        import_score=1.0,
        bpe_score=3.2,
        flagged=True,
        reason="import",
    )
    assert r.flagged
    assert r.reason == "import"


def test_build_scorer_calibrates_on_repo_hunks(tmp_path: Path, monkeypatch):
    """Using a tiny synthetic repo: build_scorer must return a scorer that
    implements score_hunk without exploding."""
    from argot_bench.score import build_scorer

    # Minimal Python repo: one file with 3 functions, each >= MIN_BODY_LINES (5)
    # of body so sample_hunks can pick 2.
    repo = tmp_path / "repo"
    repo.mkdir()
    (repo / "mod.py").write_text(
        "\n".join(
            [
                "def a():",
                "    x = 1",
                "    y = 2",
                "    z = 3",
                "    w = 4",
                "    v = 5",
                "    return x + y + z + w + v",
                "",
                "def b():",
                "    lst = [1, 2, 3]",
                "    tot = 0",
                "    for i in lst:",
                "        tot += i",
                "        tot *= 2",
                "    return tot",
                "",
                "def c():",
                "    s = 'hello'",
                "    t = s.upper()",
                "    u = t.strip()",
                "    v = u.lower()",
                "    w = v + '!'",
                "    return w",
            ]
        )
    )

    scorer = build_scorer(repo, n_cal=2, seed=0, language="python")
    result = scorer.score_hunk(
        "import requests\nrequests.get('https://example.com')",
        file_source=None,
        hunk_start_line=None,
        hunk_end_line=None,
    )
    assert "flagged" in vars(result) or hasattr(result, "flagged")


