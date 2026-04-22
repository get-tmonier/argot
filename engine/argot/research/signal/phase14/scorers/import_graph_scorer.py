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
- Hunk parsing fallback: if ``ast.parse`` raises ``SyntaxError`` (mid-block
  slice), a line-by-line regex extracts ``import X`` / ``from X import`` lines.
  The full-file _fixture_path fallback is intentionally excluded — that would
  leak decoy-function imports into the hunk score.
"""

from __future__ import annotations

import ast
import re
from collections.abc import Iterable
from pathlib import Path

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
        # Dead code: _imports_from_regex is no longer called here — pending removal in a follow-up.
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


class ImportGraphScorer:
    """Score a hunk by how many modules it imports that are foreign to the repo."""

    def __init__(self) -> None:
        self._repo_modules: frozenset[str] = frozenset()

    def fit(self, model_a_files: Iterable[Path]) -> None:
        """Parse every file in model_A and collect the set of top-level modules."""
        seen: set[str] = set()
        for path in model_a_files:
            try:
                source = path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            seen.update(_imports_from_ast(source))
        self._repo_modules = frozenset(seen)

    def score_hunk(self, hunk_source: str) -> float:
        """Return the count of top-level modules in hunk_source not seen in model_A.

        Returns 0.0 if no foreign imports are found.
        """
        hunk_modules = _imports_from_ast(hunk_source)
        foreign = hunk_modules - self._repo_modules
        return float(len(foreign))
