from __future__ import annotations

from pathlib import Path


# ---------------------------------------------------------------------------
# ScoreResult basic shape
# ---------------------------------------------------------------------------


def test_score_result_fields() -> None:
    from argot_bench.score import ScoreResult

    r = ScoreResult(
        import_score=1.0,
        bpe_score=3.2,
        flagged=True,
        reason="import",
    )
    assert r.flagged
    assert r.reason == "import"


def test_score_result_accepts_call_receiver_reason() -> None:
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


def test_score_result_default_call_receiver_unattested_is_empty() -> None:
    from argot_bench.score import ScoreResult

    r = ScoreResult(
        import_score=1.0,
        bpe_score=3.2,
        flagged=True,
        reason="import",
    )
    assert r.call_receiver_unattested == ()


# ---------------------------------------------------------------------------
# build_scorer integration tests
# ---------------------------------------------------------------------------


def test_build_scorer_calibrates_on_repo_hunks(tmp_path: Path, monkeypatch) -> None:  # type: ignore[no-untyped-def]
    """build_scorer must return a scorer that implements score_hunk."""
    from argot_bench.score import build_scorer

    repo = tmp_path / "repo"
    repo.mkdir()
    (repo / "mod.py").write_text(
        "\n".join([
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
        ])
    )

    scorer = build_scorer(repo, n_cal=2, seed=0, language="python")
    result = scorer.score_hunk(
        "import requests\nrequests.get('https://example.com')",
        file_source=None,
        hunk_start_line=None,
        hunk_end_line=None,
    )
    assert hasattr(result, "flagged")


def test_build_scorer_alpha_zero_disables_call_receiver_stage(tmp_path: Path) -> None:
    """alpha=0.0 -> no call_receiver scorer built inside inner."""
    from argot_bench.score import build_scorer

    repo = tmp_path / "repo"
    repo.mkdir()
    (repo / "mod.py").write_text(
        "def a():\n"
        "    logger.info('x')\n"
        "    logger.info('y')\n"
        "    logger.info('z')\n"
        "    logger.info('w')\n"
        "    logger.info('v')\n"
        "    return None\n"
    )

    scorer = build_scorer(repo, n_cal=1, seed=0, language="python", call_receiver_alpha=0.0)
    result = scorer.score_hunk("Math.random()")
    assert result.reason != "call_receiver"


def test_build_scorer_import_stage_takes_precedence_over_call_receiver(tmp_path: Path) -> None:
    """import stage fires before call_receiver even when alpha > 0."""
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
        repo, n_cal=1, seed=0, language="python", call_receiver_alpha=0.5
    )
    result = scorer.score_hunk("import flask\napp = flask.Flask(__name__)")
    assert result.reason == "import"
