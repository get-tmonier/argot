import json
from pathlib import Path

from argot_bench.report import (
    CorpusReport,
    render_report_md,
    write_corpus_json,
)


def test_write_corpus_json_roundtrips(tmp_path: Path):
    report = CorpusReport(
        corpus="fastapi",
        language="python",
        metrics={
            "auc_catalog": 0.98,
            "fp_rate_real_pr": 0.015,
            "threshold_cv": 0.035,
            "recall_by_category": {"async_blocking": 1.0, "routing": 0.83},
            "stage_attribution": {"import": 12, "bpe": 8, "none": 2},
            "calibration_stability": {"rel_var": 0.001, "jaccard": 0.95},
        },
        raw_scores=[],
    )
    out = tmp_path / "fastapi.json"
    write_corpus_json(report, out)

    roundtripped = json.loads(out.read_text())
    assert roundtripped["corpus"] == "fastapi"
    assert abs(roundtripped["metrics"]["auc_catalog"] - 0.98) < 1e-9


def test_render_report_md_has_all_corpora_and_metrics():
    reports = [
        CorpusReport(
            corpus="fastapi",
            language="python",
            metrics={
                "auc_catalog": 0.98,
                "fp_rate_real_pr": 0.015,
                "threshold_cv": 0.035,
                "recall_by_category": {"async_blocking": 1.0},
                "stage_attribution": {"import": 5},
                "calibration_stability": {"rel_var": 0.001, "jaccard": 0.95},
            },
            raw_scores=[],
        ),
    ]
    md = render_report_md(reports)
    assert "# argot-bench report" in md
    assert "fastapi" in md
    assert "auc_catalog" in md or "AUC" in md
    assert "async_blocking" in md
