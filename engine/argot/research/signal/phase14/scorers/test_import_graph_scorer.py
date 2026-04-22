# engine/argot/research/signal/phase14/scorers/test_import_graph_scorer.py
"""Tests for ImportGraphScorer."""

from __future__ import annotations

import textwrap
from pathlib import Path

from argot.research.signal.phase14.scorers.import_graph_scorer import (
    ImportGraphScorer,
    _imports_from_ast,
    _imports_from_regex,
)

# ---------------------------------------------------------------------------
# Unit tests for import extraction helpers
# ---------------------------------------------------------------------------


def test_imports_from_ast_plain_import() -> None:
    source = "import os\nimport sys\n"
    assert _imports_from_ast(source) == {"os", "sys"}


def test_imports_from_ast_from_import() -> None:
    source = "from sqlalchemy.orm import Session\n"
    assert _imports_from_ast(source) == {"sqlalchemy"}


def test_imports_from_ast_relative_ignored() -> None:
    source = "from . import utils\nfrom ..core import Base\n"
    assert _imports_from_ast(source) == set()


def test_imports_from_ast_top_level_only() -> None:
    source = "from faker.providers import BaseProvider\nimport os.path\n"
    assert _imports_from_ast(source) == {"faker", "os"}


def test_imports_from_ast_syntax_error_returns_empty_set() -> None:
    # Mid-block slice — not valid Python on its own; regex fallback removed, returns empty set.
    source = "    from mimesis import Person\n    x = Person()\n"
    result = _imports_from_ast(source)
    assert result == set()


def test_imports_from_ast_docstring_prose_not_extracted() -> None:
    # Regression: "Starlette" in a docstring but source is truncated mid-function
    # (ast.parse raises SyntaxError). Must return set(), not {"Starlette"}.
    source = textwrap.dedent('''\
        def app():
            """This is built on top of Starlette.
            It uses Starlette under the hood.
        ''')
    result = _imports_from_ast(source)
    assert result == set(), f"Expected empty set, got {result!r}"


def test_imports_from_regex_basic() -> None:
    source = "import threading\nfrom numpy import random\n"
    assert _imports_from_regex(source) == {"threading", "numpy"}


def test_imports_from_regex_ignores_relative() -> None:
    # Regex version does NOT see relative imports because ``from .`` doesn't
    # start with a letter — the pattern requires [A-Za-z_]
    source = "from . import helpers\nfrom ..base import Mixin\n"
    assert _imports_from_regex(source) == set()


# ---------------------------------------------------------------------------
# Integration tests for ImportGraphScorer
# ---------------------------------------------------------------------------


def _write_py(tmp_path: Path, name: str, content: str) -> Path:
    p = tmp_path / name
    p.write_text(textwrap.dedent(content), encoding="utf-8")
    return p


def test_fit_collects_repo_modules(tmp_path: Path) -> None:
    _write_py(tmp_path, "a.py", "import faker\nfrom pydantic import BaseModel\n")
    _write_py(tmp_path, "b.py", "import requests\n")
    scorer = ImportGraphScorer()
    scorer.fit(tmp_path.glob("*.py"))
    assert scorer._repo_modules == frozenset({"faker", "pydantic", "requests"})


def test_score_hunk_no_imports_returns_zero(tmp_path: Path) -> None:
    _write_py(tmp_path, "a.py", "import faker\n")
    scorer = ImportGraphScorer()
    scorer.fit(tmp_path.glob("*.py"))
    assert scorer.score_hunk("x = 1 + 2\nprint(x)\n") == 0.0


def test_score_hunk_repo_internal_returns_zero(tmp_path: Path) -> None:
    _write_py(tmp_path, "a.py", "import faker\n")
    scorer = ImportGraphScorer()
    scorer.fit(tmp_path.glob("*.py"))
    assert scorer.score_hunk("from faker import Faker\n") == 0.0


def test_score_hunk_foreign_import_returns_count(tmp_path: Path) -> None:
    _write_py(tmp_path, "a.py", "import faker\n")
    scorer = ImportGraphScorer()
    scorer.fit(tmp_path.glob("*.py"))
    # mimesis and numpy are not in repo_modules
    result = scorer.score_hunk("from mimesis import Person\nimport numpy\n")
    assert result == 2.0


def test_score_hunk_relative_import_not_flagged(tmp_path: Path) -> None:
    _write_py(tmp_path, "a.py", "import faker\n")
    scorer = ImportGraphScorer()
    scorer.fit(tmp_path.glob("*.py"))
    assert scorer.score_hunk("from . import helpers\nfrom ..base import Mixin\n") == 0.0


def test_score_hunk_unparseable_slice(tmp_path: Path) -> None:
    _write_py(tmp_path, "a.py", "import faker\n")
    scorer = ImportGraphScorer()
    scorer.fit(tmp_path.glob("*.py"))
    # Indented slice — ast.parse fails, but tree-sitter (used by PythonAdapter)
    # is error-tolerant and still extracts the import.  mimesis is foreign → 1.0.
    indented = "    from mimesis import Person\n    p = Person()\n"
    assert scorer.score_hunk(indented) == 1.0


def test_score_hunk_empty_model_a_flags_everything(tmp_path: Path) -> None:
    scorer = ImportGraphScorer()
    scorer.fit([])  # empty model_a
    assert scorer.score_hunk("import os\n") == 1.0


def test_fit_handles_unreadable_file(tmp_path: Path) -> None:
    missing = tmp_path / "ghost.py"
    scorer = ImportGraphScorer()
    scorer.fit([missing])  # file doesn't exist — should not raise
    assert scorer._repo_modules == frozenset()


def test_score_hunk_mixed_repo_and_foreign(tmp_path: Path) -> None:
    _write_py(tmp_path, "a.py", "import os\nimport faker\n")
    scorer = ImportGraphScorer()
    scorer.fit(tmp_path.glob("*.py"))
    hunk = "import os\nfrom mimesis import Person\nimport requests\n"
    # os is repo-internal; mimesis and requests are foreign
    assert scorer.score_hunk(hunk) == 2.0
