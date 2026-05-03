"""Era 12 Phase 10 — measure cluster-frequency of residuals' callees.

Premise (option 1 from the post-Phase-9 discussion): era 11's current rule
treats a callee as ``attested`` if it appears in ANY file of the hunk's
cluster — boolean union. The faker-js residuals slip through because their
key callees (``fetch``, ``Math.random``, ``crypto.randomBytes``, ...) appear
somewhere in their cluster's files (likely in tests/examples), so the
existing scorer counts them attested and contributes 0 to the unattested
budget.

Replacing this with a frequency-aware attestation — "callee is attested
only if it appears in ≥X% of cluster files" — would flag rare-but-present
callees. This script measures whether the premise actually holds:

For each residual fixture: identify its host file's cluster, list each
callee in its hunk, and report how many files in that cluster contain
each callee. If `fetch` appears in 1–2 of 13 cluster files, option 1 has
clear room. If `fetch` is in >half the cluster files, option 1 won't help.

Cheap (no model, no embedding); ~30 sec runtime.
"""

from __future__ import annotations

import json
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

import yaml

ROOT = Path("/Users/damienmeur/projects/argot")
FEATURE_DIR = ROOT / "engine" / ".era12-features"
BENCH_DATA = ROOT / "benchmarks" / "data"
BENCH_CATALOGS = ROOT / "benchmarks" / "catalogs"
CORPORA = ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]

RESIDUAL_FIXTURES = {
    "faker_js_error_flip_2", "faker_js_error_flip_3",
    "faker_js_runtime_fetch_1", "faker_js_runtime_fetch_2",
    "faker_js_runtime_fetch_3",
}


def _read(p: Path) -> str | None:
    try:
        return p.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None


def _load_manifest(corpus: str) -> dict[str, dict[str, Any]]:
    with (BENCH_CATALOGS / corpus / "manifest.yaml").open() as f:
        m = yaml.safe_load(f)
    return {fx["id"]: fx for fx in m["fixtures"]}


def iter_corpus(c: str):
    with (FEATURE_DIR / f"{c}.jsonl").open() as f:
        for line in f:
            yield json.loads(line)


def main() -> None:
    from argot.scoring.adapters.registry import get_adapter  # type: ignore[import-not-found]

    print("=== Phase 10 cluster-frequency probe ===", file=sys.stderr)

    # Build, per corpus + per cluster, the set of (file_path, callee_set).
    # Aggregate from the JSONL rows (which have cluster_id per row).
    # NOTE: cluster_id is per-row at extract time; rows from the same file
    # share the cluster id.
    per_cluster_files: dict[tuple[str, int], dict[str, frozenset[str]]] = defaultdict(dict)
    per_cluster_callee_counts: dict[tuple[str, int], Counter[str]] = defaultdict(Counter)

    for corpus in CORPORA:
        repo_dir = BENCH_DATA / corpus / ".repo"
        manifest = _load_manifest(corpus)
        # For controls: read each unique file once and extract callees on the
        # whole file. For breaks: skip (they're synthetic, don't represent
        # actual cluster membership).
        seen_files: set[str] = set()
        ext_for_corpus = ".ts" if corpus in {"hono", "ink", "faker-js"} else ".py"
        adapter = get_adapter(ext_for_corpus)

        for record in iter_corpus(corpus):
            if record.get("is_break"):
                continue
            file_path = record.get("file_path")
            cluster_id_v = record.get("features", {}).get("cluster_id")
            if file_path is None or cluster_id_v is None:
                continue
            cluster_id = int(cluster_id_v)
            cache_key = (corpus, cluster_id, file_path)
            ck_str = f"{cache_key[0]}::{cache_key[1]}::{cache_key[2]}"
            if ck_str in seen_files:
                continue
            seen_files.add(ck_str)
            full_text = _read(repo_dir / file_path)
            if full_text is None:
                continue
            callees = set(adapter.extract_callees(full_text))
            per_cluster_files[(corpus, cluster_id)][file_path] = frozenset(callees)
            for callee in callees:
                per_cluster_callee_counts[(corpus, cluster_id)][callee] += 1

    # For each residual: locate the host file's cluster and report callee
    # frequencies for the hunk's callees.
    residuals_report: dict[str, Any] = {}
    fjs_manifest = _load_manifest("faker-js")

    for fid in sorted(RESIDUAL_FIXTURES):
        if fid not in fjs_manifest:
            residuals_report[fid] = {"error": "manifest missing"}
            continue
        fx = fjs_manifest[fid]
        host_rel = fx.get("host_file")
        if host_rel is None:
            residuals_report[fid] = {"error": "no host_file"}
            continue
        # Find the cluster_id of the host file.
        host_cluster = None
        for (corpus, cid), files in per_cluster_files.items():
            if corpus == "faker-js" and host_rel in files:
                host_cluster = cid
                break
        if host_cluster is None:
            residuals_report[fid] = {
                "error": "host file not found in any cluster",
                "host_file": host_rel,
            }
            continue

        cluster_size = len(per_cluster_files[("faker-js", host_cluster)])
        cluster_counts = per_cluster_callee_counts[("faker-js", host_cluster)]

        # Read the catalog break content + extract callees from JUST the hunk.
        catalog_full = _read(BENCH_CATALOGS / "faker-js" / str(fx["file"]))
        if catalog_full is None:
            residuals_report[fid] = {"error": "catalog file missing"}
            continue
        cat_lines = catalog_full.splitlines()
        chs = int(fx["hunk_start_line"])
        che = int(fx["hunk_end_line"])
        hunk_text = "\n".join(cat_lines[chs - 1: che])
        adapter = get_adapter(".ts")
        hunk_callees = list(adapter.extract_callees(hunk_text))

        callee_freq = []
        for c in sorted(set(hunk_callees)):
            count = cluster_counts.get(c, 0)
            callee_freq.append({
                "callee": c,
                "files_in_cluster_with_callee": count,
                "cluster_size": cluster_size,
                "frequency_pct": 100 * count / cluster_size if cluster_size else 0,
            })

        residuals_report[fid] = {
            "host_file": host_rel,
            "host_cluster_id": host_cluster,
            "cluster_size": cluster_size,
            "callees_in_hunk": sorted(set(hunk_callees)),
            "per_callee_cluster_frequency": callee_freq,
        }

    # Also dump a summary of all faker-js clusters for inspection.
    fjs_clusters = {
        cid: {
            "size": len(per_cluster_files[("faker-js", cid)]),
            "files_sample": sorted(per_cluster_files[("faker-js", cid)].keys())[:10],
        }
        for (c, cid) in per_cluster_files if c == "faker-js"
    }

    out = {
        "residuals": residuals_report,
        "fjs_clusters": fjs_clusters,
    }
    print(json.dumps(out, indent=2, default=str))


if __name__ == "__main__":
    main()
