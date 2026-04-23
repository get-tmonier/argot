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


def _synthetic_python_repo(tmp_path: Path) -> Path:
    """Create a synthetic Python repo with several normal functions."""
    repo = tmp_path / "repo"
    repo.mkdir()
    parts: list[str] = []
    for i in range(6):
        parts.append(
            "\n".join(
                [
                    f"def fn_{i}(value, registry):",
                    "    items = registry.lookup(value)",
                    "    if not items:",
                    "        return None",
                    "    out = []",
                    "    for item in items:",
                    "        out.append(item.transform(value))",
                    "    return out",
                    "",
                ]
            )
        )
    (repo / "mod.py").write_text("\n".join(parts))
    return repo


class _AllAtypicalModel:
    """Stub TypicalityModel that flags every hunk as atypical."""

    def is_atypical(self, hunk: str):  # noqa: ARG002
        from argot_bench.typicality import TypicalityFeatures

        return True, 42.0, TypicalityFeatures(0.0, 0.0, 0.0, 0.0, 0)


class _NoneAtypicalModel:
    """Stub TypicalityModel that never flags."""

    def is_atypical(self, hunk: str):  # noqa: ARG002
        from argot_bench.typicality import TypicalityFeatures

        return False, 0.0, TypicalityFeatures(0.5, 5.0, 2.0, 0.5, 15)


def test_build_scorer_filter_all_atypical_raises(tmp_path: Path):
    """When the filter rejects every pool member, build_scorer must raise."""
    from argot_bench.score import build_scorer

    repo = _synthetic_python_repo(tmp_path)
    try:
        build_scorer(
            repo,
            n_cal=2,
            seed=0,
            language="python",
            typicality_model=_AllAtypicalModel(),  # type: ignore[arg-type]
        )
    except ValueError as e:
        assert "typicality filter" in str(e).lower(), str(e)
        return
    raise AssertionError("expected ValueError when every pool hunk is atypical")


def test_build_scorer_filter_none_atypical_succeeds(tmp_path: Path):
    """When the filter rejects nothing, build_scorer succeeds normally."""
    from argot_bench.score import build_scorer

    repo = _synthetic_python_repo(tmp_path)
    scorer = build_scorer(
        repo,
        n_cal=2,
        seed=0,
        language="python",
        typicality_model=_NoneAtypicalModel(),  # type: ignore[arg-type]
    )
    assert isinstance(scorer.threshold, float)
