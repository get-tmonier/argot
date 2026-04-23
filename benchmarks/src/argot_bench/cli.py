from __future__ import annotations

import argparse
import sys
from datetime import UTC, datetime
from pathlib import Path

from argot_bench.report import CorpusReport, render_report_md, write_corpus_json
from argot_bench.run import RunConfig, run_corpus
from argot_bench.targets import Target, load_targets

_ROOT = Path(__file__).resolve().parent.parent.parent
_TARGETS_YAML = _ROOT / "targets.yaml"
_CATALOGS_DIR = _ROOT / "catalogs"
_DEFAULT_DATA = _ROOT / "data"
_DEFAULT_RESULTS = _ROOT / "results"


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="argot-bench",
        description="Benchmark argot's production scorer across 6 corpora.",
    )
    sub = p.add_subparsers(dest="subcommand")

    # default subcommand = run
    p.add_argument("--corpus", type=lambda s: s.split(","), default=None)
    p.add_argument("--quick", action="store_true")
    p.add_argument("--fresh", action="store_true")
    p.add_argument("--data-dir", type=Path, default=_DEFAULT_DATA)
    p.add_argument("--results-dir", type=Path, default=_DEFAULT_RESULTS)
    p.add_argument(
        "--typicality-filter",
        choices=["on", "off"],
        default="off",
        help="Apply the AST-derived typicality filter to calibration pool and control scoring.",
    )
    p.add_argument(
        "--seeds",
        type=int,
        default=None,
        metavar="N",
        help="Cap the number of seeds actually run to the first N (default: all 5).",
    )
    p.add_argument(
        "--sample-controls",
        type=int,
        default=None,
        metavar="N",
        help=(
            "Randomly subsample N control hunks per PR before scoring. "
            "Reproducible: uses np.random.default_rng(seed) to shuffle then take first N. "
            "Sampled runs are NOT suitable as baselines."
        ),
    )

    sub.add_parser("list-corpora", help="Print the 6 corpora in targets.yaml")
    rep = sub.add_parser("report", help="Regenerate report.md from existing JSON")
    rep.add_argument("results_dir", type=Path)

    return p


def _cmd_list_corpora() -> int:
    for t in load_targets(_TARGETS_YAML):
        print(f"{t.name}\t{t.language}\t{len(t.prs)} PR(s)")
    return 0


def _cmd_regenerate_report(results_dir: Path) -> int:
    reports: list[CorpusReport] = []
    for j in sorted(results_dir.glob("*.json")):
        import json as _json

        raw = _json.loads(j.read_text())
        reports.append(
            CorpusReport(
                corpus=raw["corpus"],
                language=raw["language"],
                metrics=raw["metrics"],
                raw_scores=raw.get("raw_scores", []),
            )
        )
    (results_dir / "report.md").write_text(render_report_md(reports))
    print(f"wrote {results_dir / 'report.md'}")
    return 0


def _select_targets(targets: list[Target], filt: list[str] | None) -> list[Target]:
    if not filt:
        return targets
    by_name = {t.name: t for t in targets}
    out: list[Target] = []
    for n in filt:
        if n not in by_name:
            print(f"unknown corpus: {n}", file=sys.stderr)
            sys.exit(2)
        out.append(by_name[n])
    return out


def _run(args: argparse.Namespace) -> int:
    targets = load_targets(_TARGETS_YAML)
    selected = _select_targets(targets, args.corpus)
    ts = datetime.now(tz=UTC).strftime("%Y%m%dT%H%M%SZ")
    out_dir = args.results_dir / ts
    out_dir.mkdir(parents=True, exist_ok=True)

    _all_seeds = [0, 1, 2, 3, 4]
    seeds = _all_seeds[: args.seeds] if args.seeds is not None else _all_seeds

    reports: list[CorpusReport] = []
    for t in selected:
        print(f"[{t.name}] running...")
        cfg = RunConfig(
            corpus=t.name,
            url=t.url,
            language=t.language,
            prs=[(pr.pr, pr.sha) for pr in t.prs],
            catalog_dir=_CATALOGS_DIR / t.name,
            data_dir=args.data_dir,
            quick=args.quick,
            fresh=args.fresh,
            typicality_filter=(args.typicality_filter == "on"),
            seeds=seeds,
            sample_controls=args.sample_controls,
        )
        r = run_corpus(cfg)
        reports.append(r)
        write_corpus_json(r, out_dir / f"{t.name}.json")

    (out_dir / "report.md").write_text(render_report_md(reports))
    print(f"wrote {out_dir / 'report.md'}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.subcommand == "list-corpora":
        return _cmd_list_corpora()
    if args.subcommand == "report":
        return _cmd_regenerate_report(args.results_dir)
    return _run(args)
