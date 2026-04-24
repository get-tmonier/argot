import json
import subprocess
import sys
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from argot_bench.cli import build_parser, main


def test_parser_defaults():
    parser = build_parser()
    args = parser.parse_args([])
    assert args.corpus is None
    assert args.quick is False
    assert args.fresh is False


def test_parser_corpus_filter():
    parser = build_parser()
    args = parser.parse_args(["--corpus=fastapi,hono"])
    assert args.corpus == ["fastapi", "hono"]


def test_list_corpora_subcommand():
    exit_code = main(["list-corpora"])
    assert exit_code == 0


def test_cli_accepts_typicality_filter_flag():
    parser = build_parser()
    ns = parser.parse_args(["--no-typicality-filter", "--quick"])
    assert ns.no_typicality_filter is True

    ns_default = parser.parse_args([])
    assert ns_default.no_typicality_filter is False


def test_cli_seeds_flag():
    parser = build_parser()
    ns = parser.parse_args(["--seeds", "1"])
    assert ns.seeds == 1

    ns_default = parser.parse_args([])
    assert ns_default.seeds is None


def test_cli_sample_controls_flag():
    parser = build_parser()
    ns = parser.parse_args(["--sample-controls", "500"])
    assert ns.sample_controls == 500

    ns_default = parser.parse_args([])
    assert ns_default.sample_controls is None


def test_cli_run_one_subcommand_parses(tmp_path: Path) -> None:
    parser = build_parser()
    ns = parser.parse_args(
        [
            "run-one",
            "fastapi",
            "--out-dir",
            str(tmp_path),
            "--quick",
            "--fresh",
            "--no-typicality-filter",
            "--seeds",
            "2",
            "--sample-controls",
            "500",
        ]
    )
    assert ns.subcommand == "run-one"
    assert ns.corpus == "fastapi"
    assert ns.out_dir == tmp_path
    assert ns.quick is True
    assert ns.fresh is True
    assert ns.no_typicality_filter is True
    assert ns.seeds == 2
    assert ns.sample_controls == 500


def test_cli_run_one_unknown_corpus_exits_2(tmp_path: Path) -> None:
    exit_code = main(["run-one", "bogus", "--out-dir", str(tmp_path)])
    assert exit_code == 2


def test_cli_orchestrator_dispatches_per_corpus(tmp_path: Path) -> None:
    _FAKE_METRICS = {
        "auc_roc": 0.75,
        "break_mean": 0.6,
        "ctrl_mean": 0.4,
        "delta": 0.2,
        "threshold_p95": 0.5,
        "n_break": 10,
        "n_ctrl": 100,
    }

    recorded_calls: list[list[str]] = []

    def fake_run(cmd: list[str], **_kwargs: object) -> MagicMock:
        recorded_calls.append(cmd)
        # write the JSON the orchestrator expects to read back
        corpus = cmd[-1]
        out_dir = Path(cmd[cmd.index("--out-dir") + 1])
        out_dir.mkdir(parents=True, exist_ok=True)
        (out_dir / f"{corpus}.json").write_text(
            json.dumps(
                {
                    "corpus": corpus,
                    "language": "python",
                    "metrics": _FAKE_METRICS,
                    "raw_scores": [],
                }
            )
        )
        m = MagicMock()
        m.returncode = 0
        return m

    with patch("argot_bench.cli.subprocess.run", side_effect=fake_run):
        exit_code = main(
            ["--corpus", "rich,faker-js", "--results-dir", str(tmp_path)]
        )

    assert exit_code == 0
    assert len(recorded_calls) == 2
    assert recorded_calls[0][-1] == "rich"
    assert recorded_calls[1][-1] == "faker-js"
    for cmd in recorded_calls:
        assert "run-one" in cmd


def test_cli_orchestrator_aggregates_json(tmp_path: Path) -> None:
    _FAKE_METRICS = {
        "auc_roc": 0.75,
        "break_mean": 0.6,
        "ctrl_mean": 0.4,
        "delta": 0.2,
        "threshold_p95": 0.5,
        "n_break": 10,
        "n_ctrl": 100,
    }

    def fake_run(cmd: list[str], **_kwargs: object) -> MagicMock:
        corpus = cmd[-1]
        out_dir = Path(cmd[cmd.index("--out-dir") + 1])
        out_dir.mkdir(parents=True, exist_ok=True)
        (out_dir / f"{corpus}.json").write_text(
            json.dumps(
                {
                    "corpus": corpus,
                    "language": "python",
                    "metrics": _FAKE_METRICS,
                    "raw_scores": [],
                }
            )
        )
        m = MagicMock()
        m.returncode = 0
        return m

    with patch("argot_bench.cli.subprocess.run", side_effect=fake_run):
        exit_code = main(
            ["--corpus", "rich,faker-js", "--results-dir", str(tmp_path)]
        )

    assert exit_code == 0
    # find the timestamped subdirectory
    subdirs = [d for d in tmp_path.iterdir() if d.is_dir()]
    assert len(subdirs) == 1
    report = subdirs[0] / "report.md"
    assert report.exists()
    content = report.read_text()
    assert "rich" in content
    assert "faker-js" in content


def test_cli_accepts_call_receiver_k_flag():
    from argot_bench.cli import build_parser

    parser = build_parser()
    ns = parser.parse_args(["--call-receiver-k", "1"])
    assert ns.call_receiver_k == 1

    ns2 = parser.parse_args(["--call-receiver-k", "2"])
    assert ns2.call_receiver_k == 2

    ns3 = parser.parse_args([])
    assert ns3.call_receiver_k == 0  # default off


@pytest.mark.integration
def test_run_one_subprocess_writes_json(tmp_path: Path) -> None:
    """Spawns a real subprocess; skipped if argot-bench is not importable."""
    proc = subprocess.run(
        [sys.executable, "-m", "argot_bench", "run-one", "bogus", "--out-dir", str(tmp_path)],
        capture_output=True,
    )
    assert proc.returncode == 2
