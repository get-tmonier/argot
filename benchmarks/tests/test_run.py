from pathlib import Path

from argot_bench.run import RunConfig, _real_pr_hunks, _reservoir_sample, _score_real_hunks, run_corpus
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

    def fake_build(
        repo, *, n_cal, seed, language, bpe_model_b=None,
        enable_typicality_filter=True, call_receiver_alpha=0.0, call_receiver_cap=5,
    ):
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


def test_run_config_has_call_receiver_alpha_cap_fields():
    from pathlib import Path

    from argot_bench.run import RunConfig

    cfg = RunConfig(
        corpus="fastapi",
        url="https://example.com/fastapi",
        language="python",
        prs=[(1, "abc")],
        catalog_dir=Path("/tmp"),
        data_dir=Path("/tmp"),
        call_receiver_alpha=0.5,
        call_receiver_cap=3,
    )
    assert cfg.call_receiver_alpha == 0.5
    assert cfg.call_receiver_cap == 3

    cfg_default = RunConfig(
        corpus="fastapi",
        url="https://example.com/fastapi",
        language="python",
        prs=[(1, "abc")],
        catalog_dir=Path("/tmp"),
        data_dir=Path("/tmp"),
    )
    assert cfg_default.call_receiver_alpha == 0.0
    assert cfg_default.call_receiver_cap == 5


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
    assert cfg_default.typicality_filter is True



def test_run_sample_controls_subsample(tmp_path: Path, monkeypatch):
    """run_corpus with sample_controls=50 on a stub 200-hunk dataset returns exactly 50 records."""
    import argot_bench.run as run_mod

    def fake_clone(data_dir, corpus, url):
        r = data_dir / corpus / ".repo"
        r.mkdir(parents=True, exist_ok=True)
        (r / ".git").mkdir(exist_ok=True)
        return r

    monkeypatch.setattr(run_mod, "ensure_clone", fake_clone)
    monkeypatch.setattr(run_mod, "ensure_sha_checked_out", lambda *_a: None)
    def fake_extract(repo, out):
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text("")
        return out

    monkeypatch.setattr(run_mod, "ensure_extracted", fake_extract)

    class FakeScorer:
        threshold = 2.5
        cal_scores = [1.0]

        def score_hunk(self, *_a, **_kw):
            from argot_bench.score import ScoreResult

            return ScoreResult(import_score=0.0, bpe_score=1.0, flagged=False, reason="none")

    monkeypatch.setattr(run_mod, "build_scorer", lambda *_a, **_kw: FakeScorer())

    import argot_bench.fixtures as fx

    cat = fx.Catalog(
        corpus="fastapi",
        language="python",
        categories=["async_blocking"],
        injection_hosts=[fx.PRHost(pr=0, sha="a" * 40)],
        fixtures=[],
    )
    monkeypatch.setattr(run_mod, "load_catalog", lambda _p: cat)
    monkeypatch.setattr(run_mod, "_read_hunk_pair", lambda *_a: ("source", "hunk"))

    # 200 stub hunks
    stub_hunks = [{"file_path": f"f{i}.py", "hunk_start_line": 0, "hunk_end_line": 1} for i in range(200)]
    monkeypatch.setattr(run_mod, "_real_pr_hunks", lambda *_a: iter(stub_hunks))

    # Make the repo directory contain "f0.py" … so _score_real_hunks can read files
    repo_dir = tmp_path / "data" / "fastapi" / ".repo"
    repo_dir.mkdir(parents=True, exist_ok=True)
    for i in range(200):
        (repo_dir / f"f{i}.py").write_text("x = 1\n")

    cfg = RunConfig(
        corpus="fastapi",
        url="https://example.com/fastapi",
        language="python",
        prs=[("pr", "a" * 40)],
        catalog_dir=tmp_path / "catalogs" / "fastapi",
        data_dir=tmp_path / "data",
        n_cal=2,
        seeds=[0],
        sample_controls=50,
    )
    report = run_corpus(cfg)
    real_pr = [r for r in report.raw_scores if r.get("source") == "real_pr"]
    assert len(real_pr) == 50, f"expected 50 control records, got {len(real_pr)}"


def test_real_pr_hunks_yields_lazily(tmp_path: Path):
    import json
    import types

    ds = tmp_path / "dataset.jsonl"
    ds.write_text(
        "\n".join(
            json.dumps({"file_path": f"x{i}.py", "hunk_start_line": i, "hunk_end_line": i + 1})
            for i in range(5)
        )
    )
    gen = _real_pr_hunks(ds)
    assert isinstance(gen, types.GeneratorType)
    first = next(gen)
    assert first["file_path"] == "x0.py"


def test_reservoir_sample_is_deterministic():
    src = [{"i": i} for i in range(1000)]
    a = _reservoir_sample(iter(src), 50, seed=0)
    b = _reservoir_sample(iter(src), 50, seed=0)
    assert [x["i"] for x in a] == [x["i"] for x in b]


def test_reservoir_sample_size_exactly_n():
    src = [{"i": i} for i in range(100)]
    out = _reservoir_sample(iter(src), 20, seed=0)
    assert len(out) == 20
    assert all(x in src for x in out)


def test_reservoir_sample_shorter_than_n():
    src = [{"i": i} for i in range(10)]
    out = _reservoir_sample(iter(src), 50, seed=0)
    assert len(out) == 10
    assert {x["i"] for x in out} == set(range(10))


def test_score_real_hunks_short_circuits_path_excluded(tmp_path: Path):
    """A hunk whose file is in a test directory returns reason='excluded_path' without
    invoking the scorer."""
    repo = tmp_path / "repo"
    test_dir = repo / "test"
    test_dir.mkdir(parents=True)
    (test_dir / "foo.test.tsx").write_text("describe('x', () => { it('y', () => {}); });\n")

    class NeverCalledScorer:
        def score_hunk(self, *_a: object, **_kw: object) -> ScoreResult:
            raise AssertionError("scorer must not be called for excluded paths")

    record = {
        "file_path": "test/foo.test.tsx",
        "hunk_start_line": 0,
        "hunk_end_line": 1,
    }
    results = _score_real_hunks(
        NeverCalledScorer(),  # type: ignore[arg-type]
        [record],
        repo,
    )
    assert len(results) == 1
    r = results[0]
    assert r["reason"] == "excluded_path"
    assert r["flagged"] is False


def test_filter_stats_counts_path_exclusions(tmp_path: Path):
    """Hunks whose files fall under excluded dirs all return reason='excluded_path'."""
    repo = tmp_path / "repo"
    tests_dir = repo / "tests"
    tests_dir.mkdir(parents=True)
    (tests_dir / "a.py").write_text("pass\n")
    (tests_dir / "b.py").write_text("pass\n")

    class NeverCalledScorer:
        def score_hunk(self, *_a: object, **_kw: object) -> ScoreResult:
            raise AssertionError("scorer must not be called for excluded paths")

    records = [
        {"file_path": "tests/a.py", "hunk_start_line": 0, "hunk_end_line": 1},
        {"file_path": "tests/b.py", "hunk_start_line": 0, "hunk_end_line": 1},
    ]
    results = _score_real_hunks(
        NeverCalledScorer(),  # type: ignore[arg-type]
        records,
        repo,
    )
    assert len(results) == 2
    assert all(r["reason"] == "excluded_path" for r in results)


def test_end_to_end_call_receiver_alpha_builds_active_scorer(tmp_path: Path):
    """build_scorer with alpha=0.5 produces a scorer with call_receiver stage enabled."""
    from argot_bench.score import BenchScorer, build_scorer

    repo = tmp_path / "repo"
    repo.mkdir()
    (repo / "mod.py").write_text(
        "import logging\n"
        "logger = logging.getLogger()\n"
        "def a():\n"
        "    logger.info('x')\n"
        "    logger.debug('y')\n"
        "    logger.warning('z')\n"
        "    logger.error('w')\n"
        "    return 0\n"
    )

    scorer = build_scorer(
        repo, n_cal=1, seed=0, language="python", call_receiver_alpha=0.5
    )
    assert isinstance(scorer, BenchScorer)
    # call_receiver stage is wired in (not disabled)
    assert scorer._call_receiver is not None  # type: ignore[attr-defined]
    assert scorer._alpha == 0.5  # type: ignore[attr-defined]
