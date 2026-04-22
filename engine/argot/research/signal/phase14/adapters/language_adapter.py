# engine/argot/research/signal/phase14/adapters/language_adapter.py
"""LanguageAdapter protocol — language-specific seam for the Phase 14 scorer."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Protocol, runtime_checkable


@dataclass(frozen=True)
class RepoModules:
    """Resolved internal module identifiers for a repository.

    ``exact`` holds full specifiers (e.g. ``"lodash"``, ``"@myorg/lib"``).
    ``prefixes`` holds stripped glob-alias prefixes (e.g. ``"@/"`` from
    ``"@/*": ["src/*"]`` in tsconfig ``compilerOptions.paths``).
    """

    exact: frozenset[str]
    prefixes: frozenset[str]


@runtime_checkable
class LanguageAdapter(Protocol):
    """Pluggable language-specific logic for import extraction, filtering, and prose masking.

    All methods are pure (no I/O) except ``resolve_repo_modules`` which reads the repo
    on disk.  Implementations must be safe to call on syntactically invalid fragments
    (e.g. mid-block hunk slices) — return empty sets / False gracefully.
    """

    file_extensions: frozenset[str]

    def extract_imports(self, source: str) -> set[str]:
        """Return top-level (non-relative) module specifiers imported in *source*.

        Relative paths (``./foo``, ``../bar``) are always repo-internal and must NOT
        be returned.  On parse error return an empty set — never raise.
        """
        ...

    def resolve_repo_modules(self, repo_root: Path) -> RepoModules:
        """Return the set of known internal module names for *repo_root*.

        For Python: returns empty exact/prefixes sets (internal modules are discovered
        via ``extract_imports`` over model_A files during ``ImportGraphScorer.fit``).
        For TypeScript: reads ``package.json`` (package name + workspace names) and
        ``tsconfig.json`` ``compilerOptions.paths`` (exact aliases and glob prefixes).
        """
        ...

    def is_data_dominant(self, source: str, threshold: float = 0.65) -> bool:
        """Return True if *source* is overwhelmingly composed of static data literals.

        Must trigger on locale-style files (arrays of strings / objects in export const
        form) regardless of file path — content-based only, no path heuristics.
        """
        ...

    def is_auto_generated(self, source: str) -> bool:
        """Return True if *source* contains an auto-generation marker in its header.

        Only inspects comment nodes (not string literals) to avoid false positives.
        """
        ...

    def prose_line_ranges(self, source: str) -> frozenset[int]:
        """Return 1-indexed line numbers that are pure prose (comments / docstrings).

        Used to blank prose before BPE scoring so natural-language tokens do not
        inflate the score.  On parse error return an empty frozenset.
        """
        ...
