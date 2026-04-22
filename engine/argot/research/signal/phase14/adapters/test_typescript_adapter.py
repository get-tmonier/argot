# engine/argot/research/signal/phase14/adapters/test_typescript_adapter.py
"""Unit tests for TypeScriptAdapter — written before implementation (TDD)."""

from __future__ import annotations

import json
import textwrap
from pathlib import Path

import pytest

from argot.research.signal.phase14.adapters.typescript_adapter import TypeScriptAdapter


@pytest.fixture(scope="module")
def adapter() -> TypeScriptAdapter:
    return TypeScriptAdapter()


# ---------------------------------------------------------------------------
# extract_imports
# ---------------------------------------------------------------------------


def test_extract_imports_basic(adapter: TypeScriptAdapter) -> None:
    src = textwrap.dedent("""\
        import React from 'react';
        import { useState } from "react";
        import { helper } from './utils';
        import type { Foo } from '../types';
    """)
    mods = adapter.extract_imports(src)
    assert "react" in mods
    # Relative imports must NOT be returned
    assert "./utils" not in mods
    assert "../types" not in mods


def test_extract_imports_require(adapter: TypeScriptAdapter) -> None:
    src = "const fs = require('fs');\nconst path = require(\"path\");\n"
    mods = adapter.extract_imports(src)
    assert "fs" in mods
    assert "path" in mods


def test_extract_imports_type_only(adapter: TypeScriptAdapter) -> None:
    src = "import type { Schema } from 'zod';\nimport type { Config } from '@scope/pkg';\n"
    mods = adapter.extract_imports(src)
    assert "zod" in mods
    assert "@scope/pkg" in mods


def test_extract_imports_tsx(adapter: TypeScriptAdapter) -> None:
    src = textwrap.dedent("""\
        import React from 'react';
        import { Button } from '@radix-ui/react-button';

        export const App = () => <Button>Click</Button>;
    """)
    mods = adapter.extract_imports(src, extension=".tsx")
    assert "react" in mods
    assert "@radix-ui/react-button" in mods


def test_extract_imports_relative_excluded(adapter: TypeScriptAdapter) -> None:
    src = "import { foo } from './foo';\nimport { bar } from '../lib/bar';\n"
    mods = adapter.extract_imports(src)
    assert not mods, f"Expected empty set, got: {mods}"


# ---------------------------------------------------------------------------
# resolve_repo_modules
# ---------------------------------------------------------------------------


def test_resolve_repo_modules_monorepo(adapter: TypeScriptAdapter, tmp_path: Path) -> None:
    pkg = {
        "name": "root-pkg",
        "workspaces": ["packages/*"],
    }
    (tmp_path / "package.json").write_text(json.dumps(pkg))
    packages = tmp_path / "packages"
    packages.mkdir()
    for name in ("alpha", "beta"):
        ws = packages / name
        ws.mkdir()
        (ws / "package.json").write_text(json.dumps({"name": f"@acme/{name}"}))

    mods = adapter.resolve_repo_modules(tmp_path)
    assert "root-pkg" in mods.exact
    assert "@acme/alpha" in mods.exact
    assert "@acme/beta" in mods.exact
    assert mods.prefixes == frozenset()


def test_resolve_repo_modules_tsconfig_paths(adapter: TypeScriptAdapter, tmp_path: Path) -> None:
    (tmp_path / "package.json").write_text(json.dumps({"name": "app"}))
    tsconfig = {
        "compilerOptions": {
            "paths": {
                "@/*": ["src/*"],
                "@utils/*": ["src/utils/*"],
            }
        }
    }
    (tmp_path / "tsconfig.json").write_text(json.dumps(tsconfig))

    mods = adapter.resolve_repo_modules(tmp_path)
    assert "app" in mods.exact
    assert "@/" in mods.prefixes
    assert "@utils/" in mods.prefixes
    assert "@/*" not in mods.exact and "@/*" not in mods.prefixes


def test_resolve_glob_alias_emits_prefix(adapter: TypeScriptAdapter, tmp_path: Path) -> None:
    (tmp_path / "package.json").write_text(json.dumps({"name": "app"}))
    tsconfig = {"compilerOptions": {"paths": {"@/*": ["src/*"]}}}
    (tmp_path / "tsconfig.json").write_text(json.dumps(tsconfig))

    mods = adapter.resolve_repo_modules(tmp_path)
    assert mods.prefixes == frozenset({"@/"})
    assert "@/*" not in mods.exact


def test_resolve_exact_alias_emits_exact(adapter: TypeScriptAdapter, tmp_path: Path) -> None:
    (tmp_path / "package.json").write_text(json.dumps({"name": "app"}))
    tsconfig = {"compilerOptions": {"paths": {"@myorg/lib": ["src/lib"]}}}
    (tmp_path / "tsconfig.json").write_text(json.dumps(tsconfig))

    mods = adapter.resolve_repo_modules(tmp_path)
    assert "@myorg/lib" in mods.exact
    assert mods.prefixes == frozenset()


def test_resolve_mixed_aliases(adapter: TypeScriptAdapter, tmp_path: Path) -> None:
    (tmp_path / "package.json").write_text(json.dumps({"name": "app"}))
    tsconfig = {
        "compilerOptions": {
            "paths": {
                "@/*": ["src/*"],
                "@myorg/lib": ["src/lib"],
            }
        }
    }
    (tmp_path / "tsconfig.json").write_text(json.dumps(tsconfig))

    mods = adapter.resolve_repo_modules(tmp_path)
    assert "@/" in mods.prefixes
    assert "@myorg/lib" in mods.exact


def test_resolve_ignores_middle_wildcard(adapter: TypeScriptAdapter, tmp_path: Path) -> None:
    (tmp_path / "package.json").write_text(json.dumps({"name": "app"}))
    tsconfig = {"compilerOptions": {"paths": {"@lib/*/tests": ["src/lib/*/tests"]}}}
    (tmp_path / "tsconfig.json").write_text(json.dumps(tsconfig))

    # Must not crash; middle-wildcard key is skipped
    mods = adapter.resolve_repo_modules(tmp_path)
    assert mods.prefixes == frozenset()
    assert "@lib/*/tests" not in mods.exact


# ---------------------------------------------------------------------------
# is_data_dominant
# ---------------------------------------------------------------------------


def test_is_data_dominant_faker_locale_style(adapter: TypeScriptAdapter) -> None:
    """Critical regression: faker-js locale files must be flagged as data-dominant."""
    # 3 lines of preamble, then a large array of strings — >65% data literal lines
    names = [f"'Name{i}'" for i in range(80)]
    src = textwrap.dedent(f"""\
        // Locale data file
        export const firstName = [
          {",\n  ".join(names)},
        ];
    """)
    assert adapter.is_data_dominant(src) is True


def test_is_data_dominant_normal_code(adapter: TypeScriptAdapter) -> None:
    src = textwrap.dedent("""\
        import { useState } from 'react';

        export function Counter() {
          const [count, setCount] = useState(0);
          return count;
        }

        export function add(a: number, b: number): number {
          return a + b;
        }
    """)
    assert adapter.is_data_dominant(src) is False


# ---------------------------------------------------------------------------
# is_auto_generated
# ---------------------------------------------------------------------------


def test_is_auto_generated_generic_header(adapter: TypeScriptAdapter) -> None:
    src = "// AUTO-GENERATED FILE — do not edit\n\nexport const x = 1;\n"
    assert adapter.is_auto_generated(src) is True


def test_is_auto_generated_normal_file(adapter: TypeScriptAdapter) -> None:
    src = "// Helper utilities\n\nexport function noop() {}\n"
    assert adapter.is_auto_generated(src) is False


# ---------------------------------------------------------------------------
# prose_line_ranges
# ---------------------------------------------------------------------------


def test_prose_line_ranges_jsdoc(adapter: TypeScriptAdapter) -> None:
    src = textwrap.dedent("""\
        /**
         * Computes the sum of two numbers.
         * @param a First operand
         * @param b Second operand
         */
        export function add(a: number, b: number): number {
          return a + b;
        }
    """)
    ranges = adapter.prose_line_ranges(src)
    # Lines 1-5 are the JSDoc block → must all be in ranges
    assert {1, 2, 3, 4, 5}.issubset(ranges), f"JSDoc lines missing from ranges: {ranges}"
    # Line 6 (function signature) must NOT be prose
    assert 6 not in ranges
