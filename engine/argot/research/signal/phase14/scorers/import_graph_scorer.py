# engine/argot/research/signal/phase14/scorers/import_graph_scorer.py
"""Import-graph foreign-module scorer.

Counts the number of top-level modules imported inside a hunk that were
never seen in the repo's own source (model_A).  Score 0 = no foreign
imports; score ≥ 1 = paradigm-break signal.

Design decisions:
- Top-level module only: ``from sqlalchemy.orm import Session`` → "sqlalchemy".
- Repo modules include all third-party deps seen in model_A.  Scorer only flags
  modules *never* encountered in model_A.
- Stdlib has no special treatment: if the repo never imports ``threading``,
  a hunk importing ``threading`` is flagged.
- Relative imports (``from . import X``, ``from ..foo import Y``) are ignored.
- Hunk parsing: if parsing raises an error (mid-block slice),
  ``adapter.extract_imports`` returns an empty set.  Callers that need to detect
  imports from the file header must pass them separately — see
  ``SequentialImportBpeScorer.score_hunk``.
"""

from __future__ import annotations

import ast
import re
from collections.abc import Iterable
from pathlib import Path

from argot.research.signal.phase14.adapters.language_adapter import LanguageAdapter
from argot.research.signal.phase14.adapters.registry import get_adapter

# Matches ``import foo`` or ``import foo.bar`` (captures the leading name)
_RE_IMPORT = re.compile(r"^\s*import\s+([A-Za-z_]\w*)", re.MULTILINE)
# Matches ``from foo`` or ``from foo.bar`` but NOT ``from .foo`` (relative)
_RE_FROM_IMPORT = re.compile(r"^\s*from\s+([A-Za-z_]\w*)", re.MULTILINE)


def _top_level(module: str) -> str:
    return module.split(".")[0]


def _imports_from_ast(source: str) -> set[str]:
    try:
        tree = ast.parse(source)
    except SyntaxError:
        # Mid-block hunk slices are not valid Python — return empty set.
        # Do NOT fall back to regex: regex matches module names from prose/docstrings
        # and produces false positives.
        return set()

    modules: set[str] = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                modules.add(_top_level(alias.name))
        elif isinstance(node, ast.ImportFrom):
            if node.level and node.level > 0:
                continue  # relative import — skip
            if node.module:
                modules.add(_top_level(node.module))
    return modules


def _imports_from_regex(source: str) -> set[str]:
    modules: set[str] = set()
    for m in _RE_IMPORT.finditer(source):
        modules.add(m.group(1))
    for m in _RE_FROM_IMPORT.finditer(source):
        modules.add(m.group(1))
    return modules


def _is_foreign(spec: str, exact: frozenset[str], prefixes: frozenset[str]) -> bool:
    if spec in exact:
        return False
    if any(spec.startswith(p) for p in prefixes):
        return False
    return True


class ImportGraphScorer:
    """Score a hunk by how many modules it imports that are foreign to the repo."""

    def __init__(
        self,
        adapter: LanguageAdapter | None = None,
        repo_root: Path | None = None,
    ) -> None:
        self._adapter: LanguageAdapter = adapter if adapter is not None else get_adapter(".py")
        self._repo_root: Path | None = repo_root
        self._repo_modules: frozenset[str] = frozenset()
        self._repo_modules_prefixes: frozenset[str] = frozenset()

    def fit(self, model_a_files: Iterable[Path]) -> None:
        """Parse every file in model_A and collect the set of top-level modules."""
        seen: set[str] = set()
        for path in model_a_files:
            try:
                source = path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            seen.update(self._adapter.extract_imports(source))
        if self._repo_root is not None:
            repo_mods = self._adapter.resolve_repo_modules(self._repo_root)
            seen.update(repo_mods.exact)
            self._repo_modules_prefixes = repo_mods.prefixes
        else:
            self._repo_modules_prefixes = frozenset()
        self._repo_modules = frozenset(seen)

    def is_foreign(self, spec: str) -> bool:
        """Return True if *spec* is not a known internal module specifier."""
        return _is_foreign(spec, self._repo_modules, self._repo_modules_prefixes)

    def score_hunk(self, hunk_source: str) -> float:
        """Return the count of top-level modules in hunk_source not seen in model_A.

        Returns 0.0 if no foreign imports are found.
        """
        hunk_modules = self._adapter.extract_imports(hunk_source)
        foreign = {spec for spec in hunk_modules if self.is_foreign(spec)}
        return float(len(foreign))
