from __future__ import annotations

import subprocess
from pathlib import Path


def ensure_extracted(repo_dir: Path, out_path: Path) -> Path:
    """Run argot-extract on repo_dir → out_path (dataset.jsonl).

    Caches: if out_path already exists and is non-empty, skip the subprocess.
    """
    if out_path.exists() and out_path.stat().st_size > 0:
        return out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["uv", "run", "argot-extract", str(repo_dir), "--out", str(out_path)],
        check=True,
    )
    return out_path
