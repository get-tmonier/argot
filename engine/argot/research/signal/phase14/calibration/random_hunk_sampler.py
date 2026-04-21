# engine/argot/research/signal/phase14/calibration/random_hunk_sampler.py
"""Random hunk sampler for calibration corpus generation.

Finds top-level Python function / class definitions with at least MIN_BODY_LINES
lines and samples n of them uniformly at random using a fixed numpy seed.
"""

from __future__ import annotations

import ast
from pathlib import Path

import numpy as np

MIN_BODY_LINES: int = 5

_DEFAULT_EXCLUDE_DIRS: frozenset[str] = frozenset(
    {
        "test",
        "tests",
        "doc",
        "docs",
        "examples",
        "example",
        "migrations",
        "migration",
        "benchmarks",
        "benchmark",
        "fixtures",
        "scripts",
        "build",
        "dist",
        "__pycache__",
        ".git",
        ".tox",
        ".eggs",
    }
)


def _is_excluded(path: Path, source_dir: Path, exclude_dirs: frozenset[str]) -> bool:
    try:
        rel = path.relative_to(source_dir)
    except ValueError:
        return True
    for part in rel.parts[:-1]:
        if part in exclude_dirs or part.startswith("test"):
            return True
    name = rel.name
    return name.startswith("test_") or name == "conftest.py"


def collect_candidates(
    source_dir: Path,
    *,
    exclude_dirs: frozenset[str] | None = None,
) -> list[str]:
    """Return all qualifying hunk strings from source_dir.

    A qualifying hunk is a top-level (module-level) FunctionDef, AsyncFunctionDef,
    or ClassDef with at least MIN_BODY_LINES lines between its first and last line.
    """
    excl = exclude_dirs if exclude_dirs is not None else _DEFAULT_EXCLUDE_DIRS
    hunks: list[str] = []

    for py_file in sorted(source_dir.rglob("*.py")):
        if _is_excluded(py_file, source_dir, excl):
            continue
        try:
            source = py_file.read_text(encoding="utf-8", errors="replace")
            tree = ast.parse(source)
        except SyntaxError:
            continue

        lines = source.splitlines()
        for node in ast.iter_child_nodes(tree):
            if not isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef):
                continue
            start = node.lineno
            end = node.end_lineno
            if end is None:
                continue
            if (end - start) < MIN_BODY_LINES:
                continue
            hunks.append("\n".join(lines[start - 1 : end]))

    return hunks


def sample_hunks(
    source_dir: Path,
    n: int,
    seed: int,
    *,
    exclude_dirs: frozenset[str] | None = None,
) -> list[str]:
    """Sample n hunk strings from source_dir using a fixed numpy RNG seed.

    Raises:
        ValueError: if fewer than n qualifying hunks exist in source_dir.
    """
    candidates = collect_candidates(source_dir, exclude_dirs=exclude_dirs)
    if len(candidates) < n:
        raise ValueError(
            f"Only {len(candidates)} qualifying hunks found in {source_dir!r}, "
            f"cannot sample n={n}. Reduce n or expand source_dir."
        )
    rng = np.random.default_rng(seed)
    indices = rng.choice(len(candidates), size=n, replace=False)
    return [candidates[int(i)] for i in sorted(indices)]
