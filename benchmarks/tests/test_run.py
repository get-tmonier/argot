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

    def fake_build(repo, **_kw):
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
    reports = run_corpus(cfg)
    assert len(reports) == 1
    report = reports[0]
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
            file_path: object = None,
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
    assert cfg_default.call_receiver_alpha == 2.0
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
    reports = run_corpus(cfg)
    assert len(reports) == 1
    real_pr = [r for r in reports[0].raw_scores if r.get("source") == "real_pr"]
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


def test_strip_break_meta_ts_drops_inside_hunk_remaps_lines() -> None:
    """`// Break: ...` inside the hunk range is dropped; line numbers remap."""
    from argot_bench.run import _strip_break_meta

    catalog = (
        "import { FakerCore } from '../core';\n"  # 1
        "\n"                                        # 2
        "// Break: provider throws mid-generation.\n"  # 3 (meta-comment)
        "export class AddressProvider {\n"          # 4
        "  zipCode(): string {\n"                   # 5
        "    throw new Error('x');\n"               # 6
        "  }\n"                                     # 7
        "}\n"                                       # 8
    )
    cleaned, new_chs, new_che = _strip_break_meta(catalog, 3, 8)
    # Line 3 dropped; subsequent lines shift up by 1.
    assert "// Break:" not in cleaned
    # Original [3, 8] covered the meta-comment + 5 code lines; cleaned hunk is
    # the same code lines, now at [3, 7].
    assert (new_chs, new_che) == (3, 7)
    cleaned_lines = cleaned.splitlines()
    assert cleaned_lines[new_chs - 1].startswith("export class AddressProvider")
    assert cleaned_lines[new_che - 1] == "}"


def test_strip_break_meta_python_hashed_marker() -> None:
    """`# Break: ...` is also recognised (Python convention)."""
    from argot_bench.run import _strip_break_meta

    catalog = (
        "from foo import bar\n"          # 1
        "# Break: missing fallback.\n"   # 2
        "def f():\n"                     # 3
        "    return bar()\n"             # 4
    )
    cleaned, new_chs, new_che = _strip_break_meta(catalog, 2, 4)
    assert "# Break:" not in cleaned
    assert (new_chs, new_che) == (2, 3)
    cleaned_lines = cleaned.splitlines()
    assert cleaned_lines[new_chs - 1].startswith("def f")


def test_strip_break_meta_noop_when_no_marker() -> None:
    """Catalog without break-meta line is unchanged; range unchanged."""
    from argot_bench.run import _strip_break_meta

    catalog = "line1\nline2\nline3\n"
    cleaned, new_chs, new_che = _strip_break_meta(catalog, 1, 3)
    assert cleaned == catalog
    assert (new_chs, new_che) == (1, 3)


def test_strip_break_meta_marker_outside_range() -> None:
    """Meta-comment outside [chs, che] is still dropped (always cleaned globally)
    but the hunk range remaps to keep covering the same code lines."""
    from argot_bench.run import _strip_break_meta

    catalog = (
        "// Break: header note.\n"  # 1 (outside hunk, but still stripped)
        "import x;\n"               # 2
        "function a() {\n"          # 3
        "  return 1;\n"             # 4
        "}\n"                       # 5
    )
    cleaned, new_chs, new_che = _strip_break_meta(catalog, 3, 5)
    assert "// Break:" not in cleaned
    cleaned_lines = cleaned.splitlines()
    # Original lines 3-5 became cleaned lines 2-4 because line 1 was dropped.
    assert (new_chs, new_che) == (2, 4)
    assert cleaned_lines[new_chs - 1].startswith("function a")
    assert cleaned_lines[new_che - 1] == "}"


def test_score_fixtures_host_injection_uses_host_path_no_prose_blanking(
    tmp_path: Path,
) -> None:
    """When fixture has host_file/host_inject_at_line, the scorer is called
    with the host file path (not the catalog path) and prose-blanking is
    skipped (hunk_start_line / hunk_end_line passed as None)."""
    import argot_bench.fixtures as fx
    from argot_bench.run import _score_fixtures

    catalog_dir = tmp_path / "catalogs" / "demo"
    catalog_dir.mkdir(parents=True)
    repo_dir = tmp_path / "repo"
    repo_dir.mkdir()
    # Catalog with `// Break:` line inside the hunk range.
    catalog_file = catalog_dir / "breaks" / "break_demo.ts"
    catalog_file.parent.mkdir(parents=True)
    catalog_file.write_text(
        "import { FakerCore } from '../core';\n"  # 1
        "\n"                                       # 2
        "// Break: demo.\n"                        # 3 (meta comment, stripped)
        "export class Demo {\n"                    # 4
        "  go(): string { return 'x'; }\n"         # 5
        "}\n"                                      # 6
    )
    # Host file in the corpus repo at host_inject_at_line=10.
    host_file = repo_dir / "src" / "module.ts"
    host_file.parent.mkdir(parents=True)
    host_file.write_text("\n".join(f"line{i}" for i in range(1, 21)) + "\n")

    fixture = fx.Fixture(
        id="demo",
        file="breaks/break_demo.ts",
        category="cat",
        hunk_start_line=3,
        hunk_end_line=6,
        rationale="r",
        host_file="src/module.ts",
        host_inject_at_line=10,
    )

    captured: list[dict] = []

    class CaptureScorer:
        threshold = 2.5

        def score_hunk(self, hunk, **kw):
            captured.append({"hunk": hunk, **kw})
            return ScoreResult(import_score=0.0, bpe_score=1.0, flagged=False, reason="none")

    _score_fixtures(CaptureScorer(), catalog_dir, [fixture], repo_dir=repo_dir)
    assert len(captured) == 1
    call = captured[0]
    # File path should be the actual host file, not the catalog path.
    assert call["file_path"] == host_file
    # Prose blanking AND typicality file-level check are bypassed in the
    # routing-fix path: line bounds + file_source all passed as None.
    # The synthesized post-injection file produces garbage from
    # prose_line_ranges (tree-sitter ERROR nodes) and triggers
    # atypical_file short-circuits — neither is wanted here.
    assert call["file_source"] is None
    assert call["hunk_start_line"] is None
    assert call["hunk_end_line"] is None
    # Hunk content was re-extracted from the cleaned catalog (no `// Break:`).
    assert "// Break:" not in call["hunk"]
    assert "export class Demo" in call["hunk"]


def test_score_fixtures_falls_back_when_host_file_missing(tmp_path: Path) -> None:
    """When fixture has no host_file metadata, legacy behaviour is preserved:
    file_path is the catalog path (under repo_dir), prose blanking enabled."""
    import argot_bench.fixtures as fx
    from argot_bench.run import _score_fixtures

    catalog_dir = tmp_path / "catalogs" / "demo"
    catalog_dir.mkdir(parents=True)
    repo_dir = tmp_path / "repo"
    repo_dir.mkdir()
    (catalog_dir / "x.ts").write_text("export const x = 1;\n")

    fixture = fx.Fixture(
        id="demo",
        file="x.ts",
        category="cat",
        hunk_start_line=1,
        hunk_end_line=1,
        rationale="r",
        host_file=None,
        host_inject_at_line=None,
    )

    captured: list[dict] = []

    class CaptureScorer:
        threshold = 2.5

        def score_hunk(self, hunk, **kw):
            captured.append({"hunk": hunk, **kw})
            return ScoreResult(import_score=0.0, bpe_score=1.0, flagged=False, reason="none")

    _score_fixtures(CaptureScorer(), catalog_dir, [fixture], repo_dir=repo_dir)
    call = captured[0]
    # Legacy: file_path = repo_dir / catalog file path
    assert call["file_path"] == repo_dir / "x.ts"
    # Legacy: hunk line bounds are passed (prose blanking enabled)
    assert call["hunk_start_line"] == 1
    assert call["hunk_end_line"] == 1


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
    # call_receiver stage is wired into the inner scorer (not disabled)
    assert scorer._inner._call_receiver is not None  # type: ignore[attr-defined]
    assert scorer._inner._call_receiver.alpha == 0.5  # type: ignore[attr-defined]


def test_run_corpus_single_language_returns_one_report(tmp_path: Path, monkeypatch):
    """run_corpus with a single-language catalog always returns a one-element list."""
    import argot_bench.fixtures as fx
    import argot_bench.run as run_mod

    monkeypatch.setattr(run_mod, "ensure_clone", lambda dd, c, u: dd / c / ".repo")
    monkeypatch.setattr(run_mod, "ensure_sha_checked_out", lambda *_a: None)
    monkeypatch.setattr(
        run_mod, "ensure_extracted",
        lambda repo, out: (out.parent.mkdir(parents=True, exist_ok=True), out.write_text(""), out)[2],
    )
    monkeypatch.setattr(run_mod, "build_scorer", lambda *_a, **_kw: _make_fake_scorer())
    cat = fx.Catalog(
        corpus="fastapi",
        language="python",
        categories=["cat_a"],
        injection_hosts=[fx.PRHost(pr=0, sha="a" * 40)],
        fixtures=[],
    )
    monkeypatch.setattr(run_mod, "load_catalog", lambda _p: cat)
    monkeypatch.setattr(run_mod, "_real_pr_hunks", lambda *_a: iter([]))

    cfg = RunConfig(
        corpus="fastapi",
        url="https://example.com/fastapi",
        language="python",
        prs=[("pr", "a" * 40)],
        catalog_dir=tmp_path / "catalogs" / "fastapi",
        data_dir=tmp_path / "data",
        n_cal=2,
        seeds=[0],
    )
    reports = run_corpus(cfg)
    assert len(reports) == 1
    assert reports[0].corpus == "fastapi"
    assert reports[0].language == "python"


def test_run_corpus_multi_language_returns_two_reports(tmp_path: Path, monkeypatch):
    """run_corpus with a multi catalog returns one CorpusReport per language present."""
    import argot_bench.fixtures as fx
    import argot_bench.run as run_mod

    repo_dir = tmp_path / "data" / "dagster" / ".repo"
    repo_dir.mkdir(parents=True, exist_ok=True)
    (repo_dir / "mod.py").write_text("x = 1\n")
    (repo_dir / "mod.ts").write_text("const x = 1;\n")

    monkeypatch.setattr(run_mod, "ensure_clone", lambda dd, c, u: repo_dir)
    monkeypatch.setattr(run_mod, "ensure_sha_checked_out", lambda *_a: None)
    monkeypatch.setattr(
        run_mod, "ensure_extracted",
        lambda repo, out: (out.parent.mkdir(parents=True, exist_ok=True), out.write_text(""), out)[2],
    )
    # Patch both build_scorer (used in secondary-PR path) and build_scorers (primary path).
    monkeypatch.setattr(run_mod, "build_scorer", lambda *_a, **_kw: _make_fake_scorer())
    monkeypatch.setattr(
        run_mod,
        "build_scorers",
        lambda *_a, languages, **_kw: {lang: _make_fake_scorer() for lang in languages},
    )

    catalog_dir = tmp_path / "catalogs" / "dagster"
    catalog_dir.mkdir(parents=True)
    py_dir = catalog_dir / "breaks" / "py"
    py_dir.mkdir(parents=True)
    ts_dir = catalog_dir / "breaks" / "ts"
    ts_dir.mkdir(parents=True)
    (py_dir / "break_1.py").write_text("# line\n" * 10)
    (ts_dir / "break_1.ts").write_text("// line\n" * 10)

    cat = fx.Catalog(
        corpus="dagster",
        language="multi",
        categories=["cat_a"],
        injection_hosts=[fx.PRHost(pr=0, sha="a" * 40)],
        fixtures=[
            fx.Fixture(
                id="py_1",
                file="breaks/py/break_1.py",
                category="cat_a",
                hunk_start_line=1,
                hunk_end_line=3,
                language="python",
            ),
            fx.Fixture(
                id="ts_1",
                file="breaks/ts/break_1.ts",
                category="cat_a",
                hunk_start_line=1,
                hunk_end_line=3,
                language="typescript",
            ),
        ],
    )
    monkeypatch.setattr(run_mod, "load_catalog", lambda _p: cat)
    monkeypatch.setattr(run_mod, "_real_pr_hunks", lambda *_a: iter([]))

    cfg = RunConfig(
        corpus="dagster",
        url="https://example.com/dagster",
        language="multi",
        prs=[("pr", "a" * 40)],
        catalog_dir=catalog_dir,
        data_dir=tmp_path / "data",
        n_cal=2,
        seeds=[0],
    )
    reports = run_corpus(cfg)
    assert len(reports) == 2
    names = {r.corpus for r in reports}
    assert names == {"dagster (python)", "dagster (typescript)"}
    langs = {r.language for r in reports}
    assert langs == {"python", "typescript"}
    for r in reports:
        assert "auc_catalog" in r.metrics
        assert "recall_by_category" in r.metrics


def _make_fake_scorer():
    """Return a minimal FakeBenchScorer for use in monkeypatched tests."""
    from argot_bench.score import ScoreResult

    class _FakeScorer:
        threshold = 2.5
        cal_scores = [1.0, 2.5]
        rare_branch_fire_count = 0

        def score_hunk(self, *_a, **_kw):
            return ScoreResult(import_score=0.0, bpe_score=3.0, flagged=True, reason="bpe")

    return _FakeScorer()


# ---------------------------------------------------------------------------
# Streaming partition: per-language reservoirs over an extract stream
# ---------------------------------------------------------------------------


def test_partition_real_pr_hunks_streams_per_language() -> None:
    """The partition routes records to per-language buckets in a single pass.

    No materialisation of the full record list — the only state held is one
    bucket per language. Catches the regression where the multi-language
    bench previously did ``list(_real_pr_hunks(...))`` to filter, OOMing on
    monorepo-scale corpora.
    """
    from argot_bench.run import _partition_real_pr_hunks_by_lang

    records = [
        {"file_path": f"f{i}.py", "hunk_start_line": 1, "hunk_end_line": 2, "language": "python"}
        for i in range(10)
    ] + [
        {"file_path": f"g{i}.ts", "hunk_start_line": 1, "hunk_end_line": 2, "language": "typescript"}
        for i in range(7)
    ]
    out = _partition_real_pr_hunks_by_lang(
        iter(records), langs=["python", "typescript"], quick=False, sample_controls=None, seed=0
    )
    assert len(out["python"]) == 10
    assert len(out["typescript"]) == 7


def test_partition_real_pr_hunks_quick_caps_per_language() -> None:
    """Quick mode caps each language at 50 records — total memory is
    ``50 × n_langs`` regardless of input stream size.
    """
    from argot_bench.run import _partition_real_pr_hunks_by_lang

    records = [
        {"file_path": f"f{i}.py", "hunk_start_line": 1, "hunk_end_line": 2, "language": "python"}
        for i in range(200)
    ]
    out = _partition_real_pr_hunks_by_lang(
        iter(records), langs=["python", "typescript"], quick=True, sample_controls=None, seed=0
    )
    assert len(out["python"]) == 50
    assert len(out["typescript"]) == 0


def test_partition_real_pr_hunks_reservoir_sample() -> None:
    """When ``sample_controls`` is set, each language gets its own
    bounded reservoir of that size — never holding more than
    ``sample_controls`` records of a given language at once.
    """
    from argot_bench.run import _partition_real_pr_hunks_by_lang

    records = [
        {"file_path": f"f{i}.py", "hunk_start_line": 1, "hunk_end_line": 2, "language": "python"}
        for i in range(1000)
    ]
    out = _partition_real_pr_hunks_by_lang(
        iter(records),
        langs=["python", "typescript"],
        quick=False,
        sample_controls=50,
        seed=42,
    )
    assert len(out["python"]) == 50
    # Determinism: same seed → same sample
    out2 = _partition_real_pr_hunks_by_lang(
        iter(records),
        langs=["python", "typescript"],
        quick=False,
        sample_controls=50,
        seed=42,
    )
    assert out["python"] == out2["python"]


def test_real_pr_hunks_drops_token_arrays() -> None:
    """The projection in :func:`_real_pr_hunks` must strip
    ``hunk_tokens`` / ``context_before`` / ``context_after``.
    Those arrays each carry up to 50 lines of token-list payload;
    keeping them across 400k records on Dagster is what OOMed
    the bench at 40 GB.
    """
    import json

    from argot_bench.run import _real_pr_hunks

    rec = {
        "file_path": "a.py",
        "hunk_start_line": 1,
        "hunk_end_line": 5,
        "language": "python",
        "commit_sha": "abc",
        "hunk_tokens": [{"text": "x"}] * 1000,
        "context_before": [[{"text": "y"}] * 100] * 50,
        "context_after": [[{"text": "z"}] * 100] * 50,
    }
    p = Path(__file__).parent.parent / "data" / "test_projection.jsonl"
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(rec) + "\n")
    try:
        records = list(_real_pr_hunks(p))
        assert len(records) == 1
        kept = records[0]
        assert "file_path" in kept
        assert "hunk_start_line" in kept
        assert "language" in kept
        assert "hunk_tokens" not in kept
        assert "context_before" not in kept
        assert "context_after" not in kept
    finally:
        p.unlink(missing_ok=True)


def test_partition_real_pr_hunks_skips_unknown_language() -> None:
    """Records whose ``language`` field doesn't match any requested
    language are dropped silently — they're noise from the perspective
    of a per-language scoring pass and shouldn't pollute either bucket.
    """
    from argot_bench.run import _partition_real_pr_hunks_by_lang

    records = [
        {"file_path": "a.py", "hunk_start_line": 1, "hunk_end_line": 2, "language": "python"},
        {"file_path": "b.go", "hunk_start_line": 1, "hunk_end_line": 2, "language": "go"},
        {"file_path": "c.ts", "hunk_start_line": 1, "hunk_end_line": 2, "language": "typescript"},
        {"file_path": "d.rs", "hunk_start_line": 1, "hunk_end_line": 2},  # missing language
    ]
    out = _partition_real_pr_hunks_by_lang(
        iter(records), langs=["python", "typescript"], quick=False, sample_controls=None, seed=0
    )
    assert [r["file_path"] for r in out["python"]] == ["a.py"]
    assert [r["file_path"] for r in out["typescript"]] == ["c.ts"]


def test_partition_real_pr_hunks_quick_dominates_sample_controls() -> None:
    """When ``quick=True`` the cap is the quick limit (50), not
    ``sample_controls``. Quick mode is a stronger guarantee than
    sample_controls and the partition must reflect that — keeping a
    larger reservoir under quick would defeat the point of --quick.
    """
    from argot_bench.run import _partition_real_pr_hunks_by_lang

    records = [
        {"file_path": f"f{i}.py", "hunk_start_line": 1, "hunk_end_line": 2, "language": "python"}
        for i in range(200)
    ]
    out = _partition_real_pr_hunks_by_lang(
        iter(records),
        langs=["python", "typescript"],
        quick=True,
        sample_controls=500,  # would have allowed 200 if not for quick
        seed=0,
    )
    assert len(out["python"]) == 50


def test_partition_real_pr_hunks_empty_stream_returns_empty_buckets() -> None:
    """Empty input still yields one bucket per requested language so the
    caller can iterate without a defensive ``.get(lang, [])``.
    """
    from argot_bench.run import _partition_real_pr_hunks_by_lang

    out = _partition_real_pr_hunks_by_lang(
        iter([]), langs=["python", "typescript"], quick=False, sample_controls=None, seed=0
    )
    assert out == {"python": [], "typescript": []}


def test_partition_real_pr_hunks_bounded_memory_under_sample_controls() -> None:
    """The reservoir size must be capped at ``sample_controls`` *during*
    the streaming pass, not just at the end. Catches a regression where
    the partition would accumulate then truncate, defeating the memory
    guarantee on multi-million-record streams.
    """
    from argot_bench.run import _partition_real_pr_hunks_by_lang

    # Generator yields a million records lazily — if the partition
    # materialised them all we'd OOM the test. Bounded reservoir means
    # peak Python heap stays at ~sample_controls × n_langs.
    def stream() -> object:
        for i in range(1_000_000):
            yield {
                "file_path": f"f{i}.py",
                "hunk_start_line": 1,
                "hunk_end_line": 2,
                "language": "python" if i % 2 == 0 else "typescript",
            }

    out = _partition_real_pr_hunks_by_lang(
        stream(),
        langs=["python", "typescript"],
        quick=False,
        sample_controls=100,
        seed=0,
    )
    assert len(out["python"]) == 100
    assert len(out["typescript"]) == 100


def test_load_diff_hunks_for_probe_streams_jsonl(tmp_path: Path) -> None:
    """The probe must not load the full dataset.jsonl into memory.

    Writes a 5000-record dataset and asks for n=20 samples. Internally
    the probe does Algorithm R reservoir sampling over metadata triplets;
    the test asserts the output size is bounded and deterministic per
    seed, plus that the output references valid file paths (so the
    streaming projection didn't drop the data we need at the consumer).
    """
    import json as _json

    from argot_bench.score import _load_diff_hunks_for_probe

    repo_dir = tmp_path / "repo"
    repo_dir.mkdir()
    # Real files long enough for diverse hunk bounds across records.
    for i in range(10):
        (repo_dir / f"f{i}.py").write_text("\n".join(f"line{j}" for j in range(2000)) + "\n")

    dataset = tmp_path / "dataset.jsonl"
    # Generate 5000 records with diverse (file, hs, he) triplets so the
    # probe's dedup-by-triplet doesn't collapse the input to a handful
    # of rows — without diversity the seed-determinism check would pass
    # trivially regardless of streaming behaviour.
    with dataset.open("w") as f:
        for i in range(5000):
            file_idx = i % 10
            hs = (i // 10) % 1000  # cycles 0..999 across records for the same file
            rec = {
                "file_path": f"f{file_idx}.py",
                "hunk_start_line": hs,
                "hunk_end_line": hs + 3,
                "language": "python",
                # Bulky payload that the probe must NOT keep around for
                # records it doesn't sample.
                "hunk_tokens": [{"text": f"t{k}"} for k in range(100)],
                "context_before": [[{"text": f"cb{k}"}] for k in range(50)],
                "context_after": [[{"text": f"ca{k}"}] for k in range(50)],
            }
            f.write(_json.dumps(rec) + "\n")

    out_a = _load_diff_hunks_for_probe(dataset, repo_dir, n=20, seed=42)
    out_b = _load_diff_hunks_for_probe(dataset, repo_dir, n=20, seed=42)
    out_c = _load_diff_hunks_for_probe(dataset, repo_dir, n=20, seed=99)

    # Cap honoured — ≤ n results regardless of input size.
    assert len(out_a) <= 20
    # Determinism per seed.
    assert out_a == out_b
    # Different seeds produce different samples.
    assert out_a != out_c
    # Output shape is the (hunk_content, file_abs, file_source) triple
    # that the auto-detect probe consumer expects.
    for hunk_content, file_abs, file_source in out_a:
        assert isinstance(hunk_content, str) and hunk_content
        assert file_abs.exists()
        assert isinstance(file_source, str)
