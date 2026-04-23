from pathlib import Path

from argot_bench.run import RunConfig, _score_real_hunks, run_corpus
from argot_bench.score import ScoreResult


def test_run_corpus_stub_returns_corpus_report(tmp_path: Path, monkeypatch):
    """Smoke test: run_corpus must return a CorpusReport shaped object with
    a ``metrics`` dict and a ``raw_scores`` list, even in quick mode."""
    import argot_bench.run as run_mod

    # Stub out anything network / subprocess / scorer-heavy
    def fake_clone(data_dir, corpus, url):
        r = data_dir / corpus / ".repo"
        r.mkdir(parents=True, exist_ok=True)
        (r / ".git").mkdir(exist_ok=True)
        return r

    def fake_checkout(repo, sha):
        pass

    def fake_extract(repo, out):
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text("{}\n")
        return out

    class FakeBenchScorer:
        threshold = 2.5
        cal_scores = [1.0, 2.5]

        def score_hunk(self, *_a, **_kw):
            from argot_bench.score import ScoreResult

            return ScoreResult(import_score=0.0, bpe_score=3.0, flagged=True, reason="bpe")

    def fake_build(repo, *, n_cal, seed, language, bpe_model_b=None, typicality_model=None):
        return FakeBenchScorer()

    monkeypatch.setattr(run_mod, "ensure_clone", fake_clone)
    monkeypatch.setattr(run_mod, "ensure_sha_checked_out", fake_checkout)
    monkeypatch.setattr(run_mod, "ensure_extracted", fake_extract)
    monkeypatch.setattr(run_mod, "build_scorer", fake_build)

    # Stub fixtures with a single known break
    import argot_bench.fixtures as fx

    cat = fx.Catalog(
        corpus="fastapi",
        language="python",
        categories=["async_blocking"],
        injection_hosts=[fx.PRHost(pr=0, sha="a" * 40)],
        fixtures=[
            fx.Fixture(
                id="x",
                file="x.py",
                category="async_blocking",
                hunk_start_line=1,
                hunk_end_line=3,
                rationale="",
            )
        ],
    )
    monkeypatch.setattr(run_mod, "load_catalog", lambda _p: cat)
    monkeypatch.setattr(run_mod, "_read_hunk_pair", lambda *_a: ("source", "hunk"))
    monkeypatch.setattr(run_mod, "_real_pr_hunks", lambda *_a, **_kw: [])

    cfg = RunConfig(
        corpus="fastapi",
        url="https://example.com/fastapi",
        language="python",
        prs=[("pr", "a" * 40)],
        catalog_dir=tmp_path / "catalogs" / "fastapi",
        data_dir=tmp_path / "data",
        n_cal=2,
        seeds=[0, 1, 2],
        quick=True,
    )
    report = run_corpus(cfg)
    assert report.corpus == "fastapi"
    assert "auc_catalog" in report.metrics
    assert "recall_by_category" in report.metrics


def test_score_real_hunks_reads_file_and_converts_to_1_indexed(tmp_path: Path):
    """Regression: extract JSONL uses file_path + 0-indexed half-open line bounds.

    A prior version read non-existent keys `hunk` / `file_source` and every
    control scored 0.0 — giving trivial AUC=1.0 against non-zero break scores.
    This test pins the correct dataset schema and the 0→1-indexed conversion.
    """
    repo = tmp_path / "repo"
    (repo / "pkg").mkdir(parents=True)
    (repo / "pkg" / "mod.py").write_text(
        "import os\n"  # line 0 (0-indexed)
        "import sys\n"  # line 1
        "\n"  # line 2
        "def f():\n"  # line 3
        "    return 1\n"  # line 4
        "\n"  # line 5
    )

    captured: dict[str, object] = {}

    class CapturingScorer:
        def score_hunk(
            self,
            hunk_content: str,
            *,
            file_source: str | None,
            hunk_start_line: int | None,
            hunk_end_line: int | None,
        ) -> ScoreResult:
            captured["hunk_content"] = hunk_content
            captured["file_source"] = file_source
            captured["hunk_start_line"] = hunk_start_line
            captured["hunk_end_line"] = hunk_end_line
            return ScoreResult(import_score=0.0, bpe_score=1.23, flagged=False, reason="none")

    # Extract record shape: 0-indexed half-open [start, end). Covers lines 3..4
    # (0-indexed) = lines 4..5 (1-indexed inclusive) = the def + return.
    hunk_record = {
        "file_path": "pkg/mod.py",
        "hunk_start_line": 3,
        "hunk_end_line": 5,
        "hunk_tokens": [],
    }

    results = _score_real_hunks(CapturingScorer(), [hunk_record], repo)  # type: ignore[arg-type]

    assert len(results) == 1
    assert results[0]["bpe_score"] == 1.23
    assert captured["hunk_content"] == "def f():\n    return 1"
    assert captured["hunk_start_line"] == 4  # 1-indexed
    assert captured["hunk_end_line"] == 5  # 1-indexed inclusive
    fs = captured["file_source"]
    assert isinstance(fs, str) and "import os" in fs and "return 1" in fs


def test_score_real_hunks_skips_missing_files(tmp_path: Path):
    class NoopScorer:
        def score_hunk(self, *_a: object, **_kw: object) -> ScoreResult:
            raise AssertionError("should not be called for missing file")

    record = {"file_path": "nope.py", "hunk_start_line": 0, "hunk_end_line": 1}
    results = _score_real_hunks(NoopScorer(), [record], tmp_path)  # type: ignore[arg-type]
    assert results == []


def test_run_config_accepts_typicality_filter_field():
    from argot_bench.run import RunConfig

    cfg = RunConfig(
        corpus="x",
        url="u",
        language="python",
        prs=[(1, "a" * 40)],
        catalog_dir=Path("/tmp"),
        data_dir=Path("/tmp"),
        typicality_filter=True,
    )
    assert cfg.typicality_filter is True

    cfg_default = RunConfig(
        corpus="x",
        url="u",
        language="python",
        prs=[(1, "a" * 40)],
        catalog_dir=Path("/tmp"),
        data_dir=Path("/tmp"),
    )
    assert cfg_default.typicality_filter is False


class _FlagAllAtypical:
    """Stub TypicalityModel that flags every hunk."""

    def is_atypical(self, hunk: str):  # noqa: ARG002
        from argot_bench.typicality import TypicalityFeatures

        return True, 99.0, TypicalityFeatures(0.95, 0.0, 0.1, 0.1)


def test_score_real_hunks_short_circuits_atypical_hunks(tmp_path: Path):
    """When a typicality_model flags a hunk, the scorer is not invoked and
    the result carries reason='atypical' + distance + features."""
    repo = tmp_path / "repo"
    (repo / "pkg").mkdir(parents=True)
    (repo / "pkg" / "mod.py").write_text("def f():\n    return 1\n")

    class AssertNotCalledScorer:
        def score_hunk(self, *_a: object, **_kw: object) -> ScoreResult:
            raise AssertionError("scorer must not be invoked for atypical hunks")

    record = {
        "file_path": "pkg/mod.py",
        "hunk_start_line": 0,
        "hunk_end_line": 2,
    }
    stats: dict[str, int] = {"controls_filtered": 0}
    results = _score_real_hunks(
        AssertNotCalledScorer(),  # type: ignore[arg-type]
        [record],
        repo,
        typicality_model=_FlagAllAtypical(),  # type: ignore[arg-type]
        filter_stats=stats,
    )

    assert len(results) == 1
    r = results[0]
    assert r["flagged"] is False
    assert r["reason"] == "atypical"
    assert r["typicality_distance"] == 99.0
    assert r["typicality_features"] == [0.95, 0.0, 0.1, 0.1]
    assert stats["controls_filtered"] == 1
