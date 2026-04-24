from pathlib import Path

# ---------------------------------------------------------------------------
# ScoreResult basic shape
# ---------------------------------------------------------------------------

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


# ---------------------------------------------------------------------------
# Stub helpers used in formula tests
# ---------------------------------------------------------------------------

class _StubInner:
    """Duck-typed stub for SequentialImportBpeScorer."""

    def __init__(
        self,
        *,
        bpe_score: float,
        reason: str,
        flagged: bool,
        bpe_threshold: float,
    ) -> None:
        self.bpe_threshold = bpe_threshold
        self.cal_scores: list[float] = [0.0]
        self._bpe_score = bpe_score
        self._reason = reason
        self._flagged = flagged

    def score_hunk(
        self,
        hunk_content: str,
        *,
        file_source: str | None = None,
        hunk_start_line: int | None = None,
        hunk_end_line: int | None = None,
    ) -> dict[str, object]:
        return {
            "import_score": 0.0,
            "bpe_score": self._bpe_score,
            "flagged": self._flagged,
            "reason": self._reason,
        }


class _StubCallReceiver:
    """Duck-typed stub returning a fixed unattested count."""

    def __init__(self, count: int) -> None:
        self._count = count

    def count_unattested(self, hunk_content: str) -> int:
        return self._count


# ---------------------------------------------------------------------------
# Soft-penalty formula tests (stub-based, deterministic)
# ---------------------------------------------------------------------------

def test_soft_penalty_call_receiver_reason_when_penalty_tips_threshold():
    """adjusted_bpe > threshold but raw_bpe <= threshold -> reason=call_receiver."""
    from argot_bench.score import BenchScorer

    inner = _StubInner(bpe_score=3.0, reason="none", flagged=False, bpe_threshold=5.0)
    cr = _StubCallReceiver(count=5)  # penalty = 0.5 * min(5, 5) = 2.5 -> adjusted = 5.5 > 5.0

    scorer = BenchScorer(inner, call_receiver=cr, alpha=0.5, cap=5)  # type: ignore[arg-type]
    result = scorer.score_hunk("some hunk")

    assert result.flagged is True
    assert result.reason == "call_receiver"
    assert result.bpe_score == 3.0


def test_soft_penalty_bpe_reason_when_raw_bpe_already_over():
    """adjusted_bpe > threshold and raw_bpe > threshold -> reason=bpe."""
    from argot_bench.score import BenchScorer

    inner = _StubInner(bpe_score=6.0, reason="bpe", flagged=True, bpe_threshold=5.0)
    cr = _StubCallReceiver(count=3)  # penalty = 0.5 * 3 = 1.5, adjusted = 7.5 > 5.0

    scorer = BenchScorer(inner, call_receiver=cr, alpha=0.5, cap=5)  # type: ignore[arg-type]
    result = scorer.score_hunk("some hunk")

    assert result.flagged is True
    assert result.reason == "bpe"


def test_soft_penalty_no_flag_when_adjusted_bpe_below_threshold():
    """adjusted_bpe <= threshold -> no flag, reason=none."""
    from argot_bench.score import BenchScorer

    inner = _StubInner(bpe_score=3.0, reason="none", flagged=False, bpe_threshold=5.0)
    cr = _StubCallReceiver(count=2)  # penalty = 0.5 * 2 = 1.0 -> adjusted = 4.0 < 5.0

    scorer = BenchScorer(inner, call_receiver=cr, alpha=0.5, cap=5)  # type: ignore[arg-type]
    result = scorer.score_hunk("some hunk")

    assert result.flagged is False
    assert result.reason == "none"


def test_soft_penalty_cap_limits_penalty():
    """Cap prevents runaway penalty — 40 unattested callees caps at C=5."""
    from argot_bench.score import BenchScorer

    inner = _StubInner(bpe_score=3.0, reason="none", flagged=False, bpe_threshold=5.0)
    cr = _StubCallReceiver(count=40)  # penalty = 0.5 * min(40, 5) = 2.5 -> adjusted = 5.5 > 5.0

    scorer = BenchScorer(inner, call_receiver=cr, alpha=0.5, cap=5)  # type: ignore[arg-type]
    result = scorer.score_hunk("some hunk")

    assert result.reason == "call_receiver"


def test_soft_penalty_zero_unattested_passes_through_bpe_result():
    """n=0 -> penalty=0, adjusted_bpe = raw_bpe -> inner scorer verdict unchanged."""
    from argot_bench.score import BenchScorer

    inner = _StubInner(bpe_score=6.0, reason="bpe", flagged=True, bpe_threshold=5.0)
    cr = _StubCallReceiver(count=0)  # no unattested -> adjusted = 6.0 > 5.0

    scorer = BenchScorer(inner, call_receiver=cr, alpha=0.5, cap=5)  # type: ignore[arg-type]
    result = scorer.score_hunk("some hunk")

    assert result.flagged is True
    assert result.reason == "bpe"


def test_soft_penalty_terminal_reason_short_circuits():
    """atypical/atypical_file/excluded_path: skip call_receiver stage entirely."""
    from argot_bench.score import BenchScorer

    for terminal in ("atypical", "atypical_file", "excluded_path", "auto_generated"):
        inner = _StubInner(bpe_score=0.0, reason=terminal, flagged=False, bpe_threshold=5.0)
        cr = _StubCallReceiver(count=99)  # would tip threshold if not short-circuited

        scorer = BenchScorer(inner, call_receiver=cr, alpha=0.5, cap=5)  # type: ignore[arg-type]
        result = scorer.score_hunk("some hunk")

        assert result.reason == terminal, f"expected terminal {terminal!r}, got {result.reason!r}"
        assert result.flagged is False


def test_soft_penalty_import_reason_short_circuits():
    """import stage fired -> return unchanged, skip call_receiver stage."""
    from argot_bench.score import BenchScorer

    inner = _StubInner(bpe_score=4.0, reason="import", flagged=True, bpe_threshold=5.0)
    cr = _StubCallReceiver(count=10)  # would flip reason if not short-circuited

    scorer = BenchScorer(inner, call_receiver=cr, alpha=0.5, cap=5)  # type: ignore[arg-type]
    result = scorer.score_hunk("some hunk")

    assert result.reason == "import"
    assert result.flagged is True


def test_soft_penalty_alpha_zero_disables_stage():
    """When alpha=0.0, no penalty is applied regardless of call_receiver presence."""
    from argot_bench.score import BenchScorer

    inner = _StubInner(bpe_score=3.0, reason="none", flagged=False, bpe_threshold=5.0)
    cr = _StubCallReceiver(count=99)  # huge count, but alpha=0 so no penalty

    scorer = BenchScorer(inner, call_receiver=cr, alpha=0.0, cap=5)  # type: ignore[arg-type]
    result = scorer.score_hunk("some hunk")

    assert result.flagged is False
    assert result.reason == "none"


def test_soft_penalty_no_call_receiver_passes_through():
    """When call_receiver=None, BenchScorer returns inner scorer result unchanged."""
    from argot_bench.score import BenchScorer

    inner = _StubInner(bpe_score=3.0, reason="none", flagged=False, bpe_threshold=5.0)

    scorer = BenchScorer(inner, call_receiver=None, alpha=0.5, cap=5)  # type: ignore[arg-type]
    result = scorer.score_hunk("some hunk")

    assert result.reason == "none"
    assert result.flagged is False


# ---------------------------------------------------------------------------
# build_scorer integration tests
# ---------------------------------------------------------------------------

def test_build_scorer_calibrates_on_repo_hunks(tmp_path: Path, monkeypatch):
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


def test_build_scorer_alpha_zero_disables_call_receiver_stage(tmp_path: Path):
    """alpha=0.0 (default) -> no call_receiver object built -> stage disabled."""
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


def test_build_scorer_import_stage_takes_precedence_over_call_receiver(tmp_path: Path):
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
