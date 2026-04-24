from __future__ import annotations

import argparse
import json
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
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
        "--no-typicality-filter",
        action="store_true",
        default=False,
        help="Disable the prod typicality filter for A/B comparison (filter is on by default).",
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
    p.add_argument(
        "--call-receiver-alpha",
        type=float,
        default=1.0,
        metavar="FLOAT",
        help="Stage 1.5 soft-penalty weight. 1.0=shipping default; 0.0=off; 0.5=primary; 0.3=fallback.",
    )
    p.add_argument(
        "--call-receiver-cap",
        type=int,
        default=5,
        metavar="INT",
        help="Cap on unattested callees counted in penalty (default 5).",
    )
    p.add_argument(
        "--jobs",
        "-j",
        type=int,
        default=0,
        metavar="N",
        help="Parallel corpus workers. 0 = one per corpus (default).",
    )

    sub.add_parser("list-corpora", help="Print the 6 corpora in targets.yaml")
    rep = sub.add_parser("report", help="Regenerate report.md from existing JSON")
    rep.add_argument("results_dir", type=Path)

    one = sub.add_parser("run-one", help="Process a single corpus and exit.")
    one.add_argument("corpus", help="Corpus name from targets.yaml.")
    one.add_argument("--out-dir", type=Path, required=True)
    one.add_argument("--quick", action="store_true")
    one.add_argument("--fresh", action="store_true")
    one.add_argument("--data-dir", type=Path, default=_DEFAULT_DATA)
    one.add_argument("--no-typicality-filter", action="store_true", default=False)
    one.add_argument("--seeds", type=int, default=None)
    one.add_argument("--sample-controls", type=int, default=None)
    one.add_argument(
        "--call-receiver-alpha",
        type=float,
        default=1.0,
        metavar="FLOAT",
        help="Stage 1.5 soft-penalty weight. 1.0=shipping default; 0.0=off.",
    )
    one.add_argument(
        "--call-receiver-cap",
        type=int,
        default=5,
        metavar="INT",
        help="Cap on unattested callees counted in penalty (default 5).",
    )

    return p


def _cmd_list_corpora() -> int:
    for t in load_targets(_TARGETS_YAML):
        print(f"{t.name}\t{t.language}\t{len(t.prs)} PR(s)")
    return 0


def _cmd_regenerate_report(results_dir: Path) -> int:
    reports: list[CorpusReport] = []
    for j in sorted(results_dir.glob("*.json")):
        raw = json.loads(j.read_text())
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


_ALL_SEEDS = [0, 1, 2, 3, 4]


def _cmd_run_one(args: argparse.Namespace) -> int:
    targets = load_targets(_TARGETS_YAML)
    by_name = {t.name: t for t in targets}
    if args.corpus not in by_name:
        print(f"unknown corpus: {args.corpus}", file=sys.stderr)
        return 2
    t = by_name[args.corpus]
    seeds = _ALL_SEEDS[: args.seeds] if args.seeds is not None else _ALL_SEEDS
    cfg = RunConfig(
        corpus=t.name,
        url=t.url,
        language=t.language,
        prs=[(pr.pr, pr.sha) for pr in t.prs],
        catalog_dir=_CATALOGS_DIR / t.name,
        data_dir=args.data_dir,
        quick=args.quick,
        fresh=args.fresh,
        typicality_filter=not args.no_typicality_filter,
        seeds=seeds,
        sample_controls=args.sample_controls,
        call_receiver_alpha=args.call_receiver_alpha,
        call_receiver_cap=args.call_receiver_cap,
    )
    args.out_dir.mkdir(parents=True, exist_ok=True)
    r = run_corpus(cfg)
    write_corpus_json(r, args.out_dir / f"{t.name}.json")
    return 0


def _run(args: argparse.Namespace) -> int:
    targets = load_targets(_TARGETS_YAML)
    selected = _select_targets(targets, args.corpus)
    ts = datetime.now(tz=UTC).strftime("%Y%m%dT%H%M%SZ")
    out_dir = args.results_dir / ts
    out_dir.mkdir(parents=True, exist_ok=True)

    base_cmd = [
        sys.executable,
        "-m",
        "argot_bench",
        "run-one",
        "--out-dir",
        str(out_dir),
        "--data-dir",
        str(args.data_dir),
    ]
    if args.no_typicality_filter:
        base_cmd.append("--no-typicality-filter")
    if args.quick:
        base_cmd.append("--quick")
    if args.fresh:
        base_cmd.append("--fresh")
    if args.seeds is not None:
        base_cmd.extend(["--seeds", str(args.seeds)])
    if args.sample_controls is not None:
        base_cmd.extend(["--sample-controls", str(args.sample_controls)])
    if args.call_receiver_alpha != 1.0:
        base_cmd.extend(["--call-receiver-alpha", str(args.call_receiver_alpha)])
    if args.call_receiver_cap != 5:
        base_cmd.extend(["--call-receiver-cap", str(args.call_receiver_cap)])

    def _run_corpus_subprocess(t: Target) -> tuple[str, int, str]:
        proc = subprocess.run(
            base_cmd + [t.name], check=False, capture_output=True, text=True
        )
        return t.name, proc.returncode, proc.stdout + proc.stderr

    workers = args.jobs if args.jobs > 0 else len(selected)
    print(f"running {len(selected)} corpus/corpora with {workers} parallel worker(s)...")
    failed = False
    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures = {pool.submit(_run_corpus_subprocess, t): t for t in selected}
        for future in as_completed(futures):
            name, rc, output = future.result()
            for line in output.splitlines():
                print(f"[{name}] {line}")
            if rc != 0:
                print(f"[{name}] FAILED (exit {rc})", file=sys.stderr)
                failed = True
            else:
                print(f"[{name}] done")
    if failed:
        return 1

    reports: list[CorpusReport] = []
    for t in selected:
        j = out_dir / f"{t.name}.json"
        if not j.exists():
            print(f"[{t.name}] missing output {j}", file=sys.stderr)
            return 1
        raw = json.loads(j.read_text())
        reports.append(
            CorpusReport(
                corpus=raw["corpus"],
                language=raw["language"],
                metrics=raw["metrics"],
                raw_scores=raw.get("raw_scores", []),
            )
        )

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
    if args.subcommand == "run-one":
        return _cmd_run_one(args)
    return _run(args)
