# engine/argot/research/signal/phase14/scorers/test_sequential_import_bpe_scorer_typescript.py
"""End-to-end Stage 1 tests for SequentialImportBpeScorer on a fake TypeScript repo.

These tests verify that the LanguageAdapter hot-path wiring (Changes 1 and 2 from
language_adapter_refactor_2026-04-22.md §5) routes TypeScript hunks through the
TypeScriptAdapter correctly:
  - relative imports are never flagged (fix10 contract holds for TS)
  - package-self imports (own package.json name) are not flagged
  - tsconfig path aliases are not flagged
  - genuinely foreign npm imports ARE flagged
  - hunks with no imports at all are never flagged

Stage 2 (BPE) is irrelevant here — the fake tokenizer produces a threshold of 0.0
so only Stage 1 governs the flagged / reason fields.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from argot.research.signal.phase14.adapters.typescript_adapter import TypeScriptAdapter
from argot.research.signal.phase14.scorers.sequential_import_bpe_scorer import (
    SequentialImportBpeScorer,
)


# ---------------------------------------------------------------------------
# Fake tokenizer — identical pattern to test_sequential_import_bpe_scorer.py
# ---------------------------------------------------------------------------


class _FakeTok:
    """Tokenizer stub that returns empty IDs for all inputs."""

    def __init__(self) -> None:
        self._vocab: dict[str, int] = {f"token_{i:03d}": i for i in range(1, 10)}

    def encode(self, source: str, *, add_special_tokens: bool = False) -> list[int]:  # noqa: ARG002
        return []

    def get_vocab(self) -> dict[str, int]:
        return dict(self._vocab)


def _make_model_b(tmp_path: Path) -> Path:
    """Minimal model_b.json: one token with count 1."""
    payload = {"token_counts": {"1": 1}, "total_tokens": 1}
    p = tmp_path / "model_b.json"
    p.write_text(json.dumps(payload), encoding="utf-8")
    return p


# ---------------------------------------------------------------------------
# Fixture: fake TypeScript repo
# ---------------------------------------------------------------------------


@pytest.fixture()
def ts_repo(tmp_path: Path) -> Path:
    """Create a minimal fake TS repo layout.

    package.json  → package name "@myorg/lib"
    tsconfig.json → paths: {"@/utils": ["src/utils"]}
                    (exact key so resolve_repo_modules emits "@/utils" directly;
                    glob-style "@/*" would require prefix-matching in the scorer
                    which is deferred to a later phase)
    src/index.ts  → re-exports from ./foo
    src/foo.ts    → exports foo
    src/vendor.ts → exports bar (reachable only via resolve_repo_modules)
    """
    # package.json
    (tmp_path / "package.json").write_text(
        json.dumps({"name": "@myorg/lib"}), encoding="utf-8"
    )
    # tsconfig.json — exact alias key so exact-match lookup works
    (tmp_path / "tsconfig.json").write_text(
        json.dumps({"compilerOptions": {"paths": {"@/utils": ["src/utils"]}}}),
        encoding="utf-8",
    )
    src = tmp_path / "src"
    src.mkdir()
    (src / "index.ts").write_text("export { foo } from './foo';\n", encoding="utf-8")
    (src / "foo.ts").write_text("export const foo = 1;\n", encoding="utf-8")
    (src / "vendor.ts").write_text("export const bar = 2;\n", encoding="utf-8")
    return tmp_path


def _make_scorer(ts_repo: Path) -> SequentialImportBpeScorer:
    """Build a scorer over the fake TS repo with a fake tokenizer."""
    model_a_files = list((ts_repo / "src").glob("*.ts"))
    model_b_path = _make_model_b(ts_repo)
    adapter = TypeScriptAdapter()
    return SequentialImportBpeScorer(
        model_a_files=model_a_files,
        bpe_model_b_path=model_b_path,
        calibration_hunks=["// normal hunk\n"],
        adapter=adapter,
        repo_root=ts_repo,
        exclude_data_dominant=False,
        _tokenizer=_FakeTok(),
    )


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


def test_hunk_with_foreign_ts_import_flags(ts_repo: Path) -> None:
    """Hunk importing lodash (not in the repo) → Stage 1 fires."""
    scorer = _make_scorer(ts_repo)
    result = scorer.score_hunk(
        "import lodash from 'lodash';\n",
        file_source="import lodash from 'lodash';\nconst x = 1;\n",
    )
    assert result["flagged"] is True
    assert result["reason"] == "import"
    assert result["import_score"] >= 1.0


def test_hunk_with_repo_ts_import_does_not_flag(ts_repo: Path) -> None:
    """Hunk importing ./foo (relative path) → Stage 1 silent."""
    scorer = _make_scorer(ts_repo)
    result = scorer.score_hunk(
        "import { foo } from './foo';\n",
        file_source="import { foo } from './foo';\nconst x = foo;\n",
    )
    assert result["flagged"] is False
    assert result["import_score"] == 0.0


def test_hunk_with_tsconfig_alias_does_not_flag(ts_repo: Path) -> None:
    """Hunk importing @/utils (tsconfig path alias) → Stage 1 silent.

    resolve_repo_modules reads tsconfig.json paths and registers "@/utils" as a
    known repo module, so the scorer does not flag it as foreign.
    """
    scorer = _make_scorer(ts_repo)
    result = scorer.score_hunk(
        "import { x } from '@/utils';\n",
        file_source="import { x } from '@/utils';\nconst y = x;\n",
    )
    assert result["flagged"] is False
    assert result["import_score"] == 0.0


def test_hunk_with_package_self_import_does_not_flag(ts_repo: Path) -> None:
    """Hunk importing @myorg/lib (own package name) → Stage 1 silent.

    resolve_repo_modules reads package.json and registers "@myorg/lib" as a
    known repo module (monorepo self-import pattern).
    """
    scorer = _make_scorer(ts_repo)
    result = scorer.score_hunk(
        "import { x } from '@myorg/lib';\n",
        file_source="import { x } from '@myorg/lib';\nconst y = x;\n",
    )
    assert result["flagged"] is False
    assert result["import_score"] == 0.0


def test_hunk_with_no_imports_does_not_flag(ts_repo: Path) -> None:
    """Pure string/logic edit with no imports → Stage 1 silent (fix10 contract for TS)."""
    scorer = _make_scorer(ts_repo)
    result = scorer.score_hunk(
        "const greeting = 'hello world';\n",
        file_source="import lodash from 'lodash';\nconst greeting = 'hello world';\n",
    )
    assert result["flagged"] is False
    assert result["import_score"] == 0.0
