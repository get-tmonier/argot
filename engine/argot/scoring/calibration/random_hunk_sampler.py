# engine/argot/research/signal/phase14/calibration/random_hunk_sampler.py
"""Random hunk sampler for calibration corpus generation.

Finds top-level sampleable units (function / class / arrow-const definitions)
with at least MIN_BODY_LINES lines and samples n of them uniformly at random
using a fixed numpy seed.

Language-specific traversal is fully delegated to LanguageAdapter implementations
via ``enumerate_sampleable_ranges``.  Pass an adapter to handle any supported
language; omit it (or pass None) to default to Python.
"""

from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

import numpy as np

from argot.scoring.adapters.python_adapter import PythonAdapter

if TYPE_CHECKING:
    from argot.scoring.adapters.language_adapter import LanguageAdapter

MIN_BODY_LINES: int = 5

DEFAULT_EXCLUDE_DIRS: frozenset[str] = frozenset(
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
_DEFAULT_EXCLUDE_DIRS = DEFAULT_EXCLUDE_DIRS


def is_excluded_path(path: Path, source_dir: Path, exclude_dirs: frozenset[str]) -> bool:
    try:
        rel = path.relative_to(source_dir)
    except ValueError:
        return True
    for part in rel.parts[:-1]:
        if part in exclude_dirs or part.startswith("test") or part == "__tests__":
            return True
    name = rel.name
    # Python test files
    if name.startswith("test_") or name == "conftest.py":
        return True
    # TypeScript/JavaScript test files: foo.test.ts, foo.spec.tsx, etc.
    return ".test." in name or ".spec." in name


_is_excluded = is_excluded_path


def collect_candidates(
    source_dir: Path,
    *,
    exclude_dirs: frozenset[str] | None = None,
    exclude_auto_generated: bool = True,
    exclude_data_dominant: bool = True,
    adapter: "LanguageAdapter | None" = None,  # noqa: UP037
) -> list[str]:
    """Return all qualifying hunk strings from source_dir.

    A qualifying hunk is a top-level sampleable unit (as returned by
    ``adapter.enumerate_sampleable_ranges``) with at least MIN_BODY_LINES lines.

    Args:
        adapter: LanguageAdapter implementation to use.  Defaults to PythonAdapter.
        exclude_auto_generated: When True (default), skip auto-generated files.
        exclude_data_dominant: When True (default), skip data-dominant files.
    """
    excl = exclude_dirs if exclude_dirs is not None else DEFAULT_EXCLUDE_DIRS
    _adapter: LanguageAdapter = adapter if adapter is not None else PythonAdapter()
    hunks: list[str] = []

    for ext in _adapter.file_extensions:
        for src_file in sorted(source_dir.rglob(f"*{ext}")):
            if is_excluded_path(src_file, source_dir, excl):
                continue
            try:
                source = src_file.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue

            if exclude_auto_generated and _adapter.is_auto_generated(source):
                continue
            if exclude_data_dominant and _adapter.is_data_dominant(source):
                continue

            lines = source.splitlines()
            for start, end in _adapter.enumerate_sampleable_ranges(source):
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
    exclude_auto_generated: bool = True,
    exclude_data_dominant: bool = True,
    adapter: "LanguageAdapter | None" = None,  # noqa: UP037
) -> list[str]:
    """Sample n hunk strings from source_dir using a fixed numpy RNG seed.

    Raises:
        ValueError: if fewer than n qualifying hunks exist in source_dir.
    """
    candidates = collect_candidates(
        source_dir,
        exclude_dirs=exclude_dirs,
        exclude_auto_generated=exclude_auto_generated,
        exclude_data_dominant=exclude_data_dominant,
        adapter=adapter,
    )
    if len(candidates) < n:
        raise ValueError(
            f"Only {len(candidates)} qualifying hunks found in {source_dir!r}, "
            f"cannot sample n={n}. Reduce n or expand source_dir."
        )
    rng = np.random.default_rng(seed)
    indices = rng.choice(len(candidates), size=n, replace=False)
    return [candidates[int(i)] for i in sorted(indices)]


def sample_hunks_disjoint(
    source_dir: Path,
    n_cal: int,
    n_ctrl: int,
    seed: int,
    *,
    exclude_dirs: frozenset[str] | None = None,
    exclude_auto_generated: bool = True,
    exclude_data_dominant: bool = True,
    adapter: "LanguageAdapter | None" = None,  # noqa: UP037
) -> tuple[list[str], list[str]]:
    """Sample two disjoint hunk sets from source_dir using a fixed numpy RNG seed.

    Returns:
        (cal_hunks, ctrl_hunks)

    Raises:
        ValueError: if fewer than n_cal + n_ctrl qualifying hunks exist.
    """
    candidates = collect_candidates(
        source_dir,
        exclude_dirs=exclude_dirs,
        exclude_auto_generated=exclude_auto_generated,
        exclude_data_dominant=exclude_data_dominant,
        adapter=adapter,
    )
    needed = n_cal + n_ctrl
    if len(candidates) < needed:
        raise ValueError(
            f"Only {len(candidates)} qualifying hunks found in {source_dir!r}, "
            f"cannot sample n_cal={n_cal} + n_ctrl={n_ctrl}={needed}. "
            f"Reduce counts or expand source_dir."
        )
    rng = np.random.default_rng(seed)
    perm = rng.permutation(len(candidates))
    cal_indices = perm[:n_cal]
    ctrl_indices = perm[n_cal : n_cal + n_ctrl]
    return (
        [candidates[int(i)] for i in cal_indices],
        [candidates[int(i)] for i in ctrl_indices],
    )
