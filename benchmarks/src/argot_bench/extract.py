from __future__ import annotations

import subprocess
from pathlib import Path

# Default cap on records per per-PR extract output. The bench needs a
# sample of a PR's diff hunks for FP-rate / control population — not the
# full git history. argot-extract walks history end-to-end by default, so
# on monorepos like Dagster one PR's dataset.jsonl can exceed 16 GB
# (~400k records × ~37 KB each, with hunk_tokens + context_before/after
# token arrays). With ``--limit`` extract early-terminates once N
# records are emitted; small corpora (faker, fastapi, …) emit far less
# than the cap and are unaffected.
DEFAULT_EXTRACT_LIMIT = 10000


def ensure_extracted(
    repo_dir: Path, out_path: Path, *, limit: int | None = DEFAULT_EXTRACT_LIMIT
) -> Path:
    """Run argot-extract on repo_dir → out_path (dataset.jsonl).

    Caches: if out_path already exists and is non-empty, skip the subprocess.
    Pass ``limit=None`` to disable the per-PR record cap (full git
    history); the default cap of :data:`DEFAULT_EXTRACT_LIMIT` keeps
    monorepo-class repos from emitting multi-GB JSONLs that downstream
    streaming can't claw back the parse cost from.
    """
    if out_path.exists() and out_path.stat().st_size > 0:
        return out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    cmd = ["uv", "run", "argot-extract", str(repo_dir), "--out", str(out_path)]
    if limit is not None:
        cmd.extend(["--limit", str(limit)])
    subprocess.run(cmd, check=True)
    return out_path
