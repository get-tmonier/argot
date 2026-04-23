# benchmarks/tests/test_cli_quick.py
import os
from pathlib import Path

import pytest

from argot_bench.cli import main


@pytest.mark.skipif(
    os.environ.get("ARGOT_BENCH_RUN_E2E") != "1",
    reason="set ARGOT_BENCH_RUN_E2E=1 to run the end-to-end smoke test (~3 min, requires network)",
)
def test_argot_bench_quick_fastapi_end_to_end(tmp_path: Path):
    exit_code = main(
        [
            "--quick",
            "--corpus=fastapi",
            f"--data-dir={tmp_path / 'data'}",
            f"--results-dir={tmp_path / 'results'}",
        ]
    )
    assert exit_code == 0
    runs = list((tmp_path / "results").iterdir())
    assert len(runs) == 1
    report_md = runs[0] / "report.md"
    assert report_md.exists()
    content = report_md.read_text()
    assert "fastapi" in content
    assert "AUC" in content or "auc_catalog" in content
    corpus_json = runs[0] / "fastapi.json"
    assert corpus_json.exists()
