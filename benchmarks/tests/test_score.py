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


def test_score_result_accepts_call_receiver_reason():
    from argot_bench.score import ScoreResult

    r = ScoreResult(
        import_score=0.0,
        bpe_score=0.0,
        flagged=True,
        reason="call_receiver",
        call_receiver_unattested=("Math.random",),
    )
    assert r.reason == "call_receiver"
    assert r.call_receiver_unattested == ("Math.random",)


def test_score_result_default_call_receiver_unattested_is_empty():
    from argot_bench.score import ScoreResult

    r = ScoreResult(
        import_score=1.0,
        bpe_score=3.2,
        flagged=True,
        reason="import",
    )
    assert r.call_receiver_unattested == ()


def test_bench_scorer_call_receiver_stage_runs_after_import_before_bpe(tmp_path):
    """Precedence: typicality > import > call_receiver > bpe > none."""
    from argot_bench.score import build_scorer

    repo = tmp_path / "repo"
    repo.mkdir()
    (repo / "mod.py").write_text(
        "\n".join([
            "import logging",
            "logger = logging.getLogger()",
            "def a():",
            "    logger.info('x')",
            "    logger.debug('y')",
            "    logger.warning('z')",
            "    logger.error('w')",
            "    return None",
        ])
    )

    scorer = build_scorer(
        repo, n_cal=1, seed=0, language="python", call_receiver_k=1
    )

    # Hunk 2: introduces Math.random — unattested → call_receiver
    r2 = scorer.score_hunk("x = Math.random()")
    assert r2.reason == "call_receiver"
    assert "Math.random" in r2.call_receiver_unattested

    # Hunk 3: imports flask — import stage fires first
    r3 = scorer.score_hunk("import flask\napp = flask.Flask(__name__)")
    assert r3.reason == "import"


def test_bench_scorer_call_receiver_k0_disables_stage(tmp_path):
    from argot_bench.score import build_scorer

    repo = tmp_path / "repo"
    repo.mkdir()
    (repo / "mod.py").write_text(
        "def a():\n    logger.info('x')\n    logger.info('y')\n    logger.info('z')\n    logger.info('w')\n    logger.info('v')\n    return None\n"
    )

    scorer = build_scorer(
        repo, n_cal=1, seed=0, language="python", call_receiver_k=0
    )

    r = scorer.score_hunk("Math.random()")
    assert r.reason != "call_receiver"
