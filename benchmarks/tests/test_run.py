from pathlib import Path

from argot_bench.run import RunConfig, run_corpus


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

    def fake_build(repo, *, n_cal, seed, language, bpe_model_b=None):
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
