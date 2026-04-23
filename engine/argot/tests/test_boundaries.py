"""Boundary enforcement tests: research imports must never appear in production code."""

from __future__ import annotations

import ast
import re
from pathlib import Path

# Resolve relative to this file: engine/argot/tests/ -> engine/argot/
_PROD_ROOT = Path(__file__).parent.parent
_SKIP_DIRS = {
    _PROD_ROOT / "tests",
    _PROD_ROOT / "acceptance" / "catalog",
    _PROD_ROOT / "research",
}

_JEPA_MODULE_PATTERN = re.compile(
    r"\b(argot\.jepa|argot\.validate|JEPAArgot|TokenEncoder|ArgotPredictor)\b"
)

_RESEARCH_MODULE_PREFIXES = ("argot.research", "research")


def test_no_research_or_jepa_imports_in_prod() -> None:
    # research/ must not exist on main — assert its absence as a hard invariant
    research_dir = _PROD_ROOT / "research"
    assert not research_dir.exists(), (
        "engine/argot/research/ must not exist on main — "
        "research code lives on its branch, never on main"
    )

    violations: list[str] = []

    for py_file in _PROD_ROOT.rglob("*.py"):
        if any(py_file.is_relative_to(skip) for skip in _SKIP_DIRS):
            continue

        source = py_file.read_text(encoding="utf-8")

        # AST import walk for research module prefixes
        try:
            tree = ast.parse(source, filename=str(py_file))
        except SyntaxError as exc:
            violations.append(f"{py_file}: SyntaxError — {exc}")
            continue

        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    for prefix in _RESEARCH_MODULE_PREFIXES:
                        if alias.name == prefix or alias.name.startswith(f"{prefix}."):
                            violations.append(
                                f"{py_file}:{node.lineno}: "
                                f"import {alias.name!r} matches research prefix"
                            )
            elif isinstance(node, ast.ImportFrom):
                module = node.module or ""
                for prefix in _RESEARCH_MODULE_PREFIXES:
                    if module == prefix or module.startswith(f"{prefix}."):
                        violations.append(
                            f"{py_file}:{node.lineno}: "
                            f"from {module!r} import ... matches research prefix"
                        )

        # Regex scan for JEPA-era names (catches string references too)
        for lineno, line in enumerate(source.splitlines(), start=1):
            if _JEPA_MODULE_PATTERN.search(line):
                violations.append(f"{py_file}:{lineno}: JEPA-era name in line: {line.strip()!r}")

    assert not violations, (
        "Production code contains forbidden research/JEPA references:\n"
        + "\n".join(f"  {v}" for v in violations)
    )


def test_research_dir_absent_on_main() -> None:
    # research code lives on its branch, never on main
    research_dir = _PROD_ROOT / "research"
    assert not research_dir.exists(), (
        f"{research_dir} must not exist on main — "
        "research code lives on its branch, never on main"
    )
