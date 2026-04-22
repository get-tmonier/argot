from __future__ import annotations

from pathlib import Path

import pytest

from argot.research.signal.phase14.parsers import PythonTreeSitterParser
from argot.research.signal.phase14.scorers.import_graph_scorer import _imports_from_ast

FASTAPI_ROOT = Path("/Users/damienmeur/projects/argot/.argot/research/repos/fastapi")

pytestmark = pytest.mark.skipif(
    not FASTAPI_ROOT.exists(), reason="fastapi repo not found"
)


def _collect_py_files() -> list[Path]:
    if not FASTAPI_ROOT.exists():
        return []
    return list(FASTAPI_ROOT.rglob("*.py"))


@pytest.mark.parametrize("path", _collect_py_files())
def test_parity_with_ast(path: Path) -> None:
    src = path.read_text(encoding="utf-8", errors="replace")
    parser = PythonTreeSitterParser()
    ast_result = _imports_from_ast(src)
    ts_result = parser.extract_imports(src)

    assert set(ast_result) == set(ts_result), (
        f"Divergence in {path}\n"
        f"  AST only : {sorted(ast_result - ts_result)}\n"
        f"  TS only  : {sorted(ts_result - ast_result)}"
    )
