import json
from pathlib import Path

from argot_bench.report import (
    CorpusReport,
    render_report_md,
    write_corpus_json,
)


def _sample_report() -> CorpusReport:
    return CorpusReport(
        corpus="fastapi",
        language="python",
        metrics={
            "auc_catalog": 0.9931,
            "fp_rate_real_pr": 0.0,
            "threshold_cv": 0.035,
            "threshold_mean": 4.1234,
            "thresholds": [4.1, 4.12, 4.14, 4.13, 4.13],
            "recall_by_category": {"async_blocking": 1.0, "routing": 0.0},
            "stage_attribution": {"bpe": 8, "none": 1},
            "calibration_stability": {"rel_var": 0.001, "jaccard": 0.95},
            "n_fixtures": 2,
            "n_real_pr_hunks": 3,
        },
        raw_scores=[
            {
                "id": "async_blocking_1",
                "category": "async_blocking",
                "file": "break.py",
                "hunk_start_line": 10,
                "hunk_end_line": 15,
                "rationale": "sync sleep inside async endpoint",
                "bpe_score": 5.25,
                "import_score": 0.0,
                "flagged": True,
                "reason": "bpe",
            },
            {
                "id": "routing_1",
                "category": "routing",
                "file": "break.py",
                "hunk_start_line": 20,
                "hunk_end_line": 25,
                "rationale": "flask-style @app.route",
                "bpe_score": 3.55,
                "import_score": 0.0,
                "flagged": False,
                "reason": "none",
            },
            {
                "source": "real_pr",
                "file_path": "app/foo.py",
                "hunk_start_line": 1,
                "hunk_end_line": 5,
                "bpe_score": 3.81,
                "import_score": 0.0,
                "flagged": False,
                "reason": "none",
            },
            {
                "source": "real_pr",
                "file_path": "app/bar.py",
                "hunk_start_line": 30,
                "hunk_end_line": 35,
                "bpe_score": 1.5,
                "import_score": 0.0,
                "flagged": False,
                "reason": "none",
            },
            {
                "source": "real_pr",
                "file_path": "app/baz.py",
                "hunk_start_line": 1,
                "hunk_end_line": 3,
                "bpe_score": -0.6,
                "import_score": 0.0,
                "flagged": False,
                "reason": "none",
            },
        ],
    )


def test_write_corpus_json_roundtrips(tmp_path: Path):
    report = _sample_report()
    out = tmp_path / "fastapi.json"
    write_corpus_json(report, out)

    roundtripped = json.loads(out.read_text())
    assert roundtripped["corpus"] == "fastapi"
    assert abs(roundtripped["metrics"]["auc_catalog"] - 0.9931) < 1e-9
    assert roundtripped["raw_scores"][0]["id"] == "async_blocking_1"


def test_render_report_md_headline_has_gap_and_sample_sizes():
    md = render_report_md([_sample_report()])
    assert "# argot-bench report" in md
    assert "## Headline" in md
    # N_fix / N_ctrl / Gap columns
    assert "N_fix" in md
    assert "N_ctrl" in md
    assert "Gap" in md
    # Gap = min_break (3.55) - max_ctrl (3.81) = -0.26 (overlap)
    assert "-0.260" in md or "-0.26" in md


def test_render_report_md_has_score_distribution_and_threshold_annotation():
    md = render_report_md([_sample_report()])
    assert "### Score distribution" in md
    # both series appear
    assert "Break (catalog)" in md
    assert "Control (real PR)" in md
    # threshold annotation
    assert "Threshold" in md
    assert "4.1234" in md


def test_render_report_md_has_per_fixture_table_with_rationale():
    md = render_report_md([_sample_report()])
    assert "### Per-fixture results" in md
    assert "async_blocking_1" in md
    assert "routing_1" in md
    # rationale
    assert "sync sleep inside async endpoint" in md
    # flagged indicators
    assert "✓" in md and "✗" in md


def test_render_report_md_callout_missed_fixtures():
    md = render_report_md([_sample_report()])
    assert "Missed fixtures" in md
    # routing_1 is unflagged with score 3.55, below threshold 4.1234
    assert "routing_1" in md
    # shows distance below threshold
    assert "below threshold" in md


def test_render_report_md_top_controls_section():
    md = render_report_md([_sample_report()])
    assert "Top" in md and "real-PR controls" in md
    # Highest control (3.81) appears before lowest (-0.6)
    top_idx = md.find("Top")
    assert md.find("3.810", top_idx) < md.find("-0.600", top_idx)


def test_render_report_md_per_category_table():
    md = render_report_md([_sample_report()])
    assert "### Per-category detail" in md
    # per-category rows
    assert "async_blocking" in md and "routing" in md
    # hits/total column
    assert "1/1" in md and "0/1" in md


def test_render_report_md_stage_attribution_has_percentages():
    md = render_report_md([_sample_report()])
    assert "Stage attribution" in md
    # 8/9 = 88.9%, 1/9 = 11.1%
    assert "88.9%" in md
    assert "11.1%" in md
