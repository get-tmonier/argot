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


# ---------------------------------------------------------------------------
# Fix A: enumerate_sampleable_ranges
# ---------------------------------------------------------------------------


def test_enumerate_sampleable_ranges_typescript_function_declaration(
    adapter: TypeScriptAdapter,
) -> None:
    src = textwrap.dedent("""\
        function greet(name: string): string {
          const prefix = "Hello";
          const suffix = "!";
          return `${prefix}, ${name}${suffix}`;
        }
    """)
    ranges = adapter.enumerate_sampleable_ranges(src)
    assert len(ranges) == 1
    assert ranges[0][0] == 1
    assert ranges[0][1] == 5


def test_enumerate_sampleable_ranges_typescript_arrow_const_export(
    adapter: TypeScriptAdapter,
) -> None:
    """export const foo = () => { ... } must produce a range."""
    src = textwrap.dedent("""\
        export const handler = (req: Request): Response => {
          const body = req.body;
          const result = process(body);
          const status = 200;
          return new Response(result, { status });
        };
    """)
    ranges = adapter.enumerate_sampleable_ranges(src)
    assert len(ranges) == 1
    assert ranges[0][0] == 1


def test_enumerate_sampleable_ranges_typescript_class_declaration(
    adapter: TypeScriptAdapter,
) -> None:
    src = textwrap.dedent("""\
        class Counter {
          private count = 0;
          increment() { this.count++; }
          decrement() { this.count--; }
          value() { return this.count; }
        }
    """)
    ranges = adapter.enumerate_sampleable_ranges(src)
    assert len(ranges) == 1
    assert ranges[0][0] == 1


def test_enumerate_sampleable_ranges_typescript_interface_type_alias(
    adapter: TypeScriptAdapter,
) -> None:
    src = textwrap.dedent("""\
        interface Config {
          host: string;
          port: number;
          timeout: number;
        }

        type Handler = (req: Request) => Response;
    """)
    ranges = adapter.enumerate_sampleable_ranges(src)
    # Both interface and type_alias_declaration should appear
    assert len(ranges) == 2


def test_enumerate_sampleable_ranges_typescript_nested_not_double_counted(
    adapter: TypeScriptAdapter,
) -> None:
    """A nested function inside a top-level function must NOT produce a separate range."""
    src = textwrap.dedent("""\
        function outer(x: number): number {
          function inner(y: number): number {
            return y * 2;
          }
          return inner(x) + 1;
        }
    """)
    ranges = adapter.enumerate_sampleable_ranges(src)
    assert len(ranges) == 1, f"Expected 1 range (outer only), got {len(ranges)}: {ranges}"


# ---------------------------------------------------------------------------
# Fix B: is_auto_generated restricted to header (HEADER_LINE_LIMIT=20)
# ---------------------------------------------------------------------------


def test_autogen_header_comment_still_flagged_ts(adapter: TypeScriptAdapter) -> None:
    src = "// AUTO-GENERATED FILE. DO NOT EDIT.\n\nexport const x = 1;\n"
    assert adapter.is_auto_generated(src) is True


def test_autogen_body_comment_mentioning_generated_not_flagged_ts(
    adapter: TypeScriptAdapter,
) -> None:
    """JSDoc comment beyond line 20 saying 'generated' must NOT flag."""
    # 20 blank lines pushes the comment to line 21 (0-indexed: 20 >= HEADER_LINE_LIMIT=20)
    body = "/** This function generates streaming HTML. */\nexport function stream() {}\n"
    src = "\n" * 20 + body
    assert adapter.is_auto_generated(src) is False


def test_autogen_marker_at_line_20_still_flagged_ts(adapter: TypeScriptAdapter) -> None:
    """Marker at line 20 (1-indexed) must still flag."""
    # 19 blank lines → comment at line 20 (0-indexed: 19 < 20) → flagged
    src = "\n" * 19 + "// auto-generated — do not edit\n"
    assert adapter.is_auto_generated(src) is True


def test_autogen_marker_at_line_21_not_flagged_ts(adapter: TypeScriptAdapter) -> None:
    """Marker at line 21 (1-indexed) must NOT flag."""
    # 20 blank lines → comment at line 21 (0-indexed: 20 >= 20) → not flagged
    src = "\n" * 20 + "// auto-generated — do not edit\n"
    assert adapter.is_auto_generated(src) is False


# ---------------------------------------------------------------------------
# Fix C: value-type-aware is_data_dominant
# ---------------------------------------------------------------------------


def test_data_dominant_array_of_strings_ts(adapter: TypeScriptAdapter) -> None:
    """Regression: faker-js locale style array of strings must still be flagged."""
    names = [f"'name_{i}'" for i in range(60)]
    src = "export const firstName = [\n  " + ",\n  ".join(names) + ",\n];\n"
    assert adapter.is_data_dominant(src) is True


def test_data_dominant_object_of_arrow_functions_ts_not_flagged(
    adapter: TypeScriptAdapter,
) -> None:
    """Object whose values are arrow functions (Hono children.ts pattern) must NOT flag."""
    src = textwrap.dedent("""\
        import type { FC } from 'hono/jsx';

        export const Children: Record<string, FC> = {
          Head: ({ children }) => <head>{children}</head>,
          Body: ({ children }) => <body>{children}</body>,
          Title: ({ children }) => <title>{children}</title>,
          Script: ({ src }) => <script src={src} />,
          Link: ({ rel, href }) => <link rel={rel} href={href} />,
          Meta: ({ name, content }) => <meta name={name} content={content} />,
          Style: ({ children }) => <style>{children}</style>,
          Html: ({ children }) => <html>{children}</html>,
        };
    """)
    assert adapter.is_data_dominant(src) is False


def test_data_dominant_object_of_strings_ts_flagged(adapter: TypeScriptAdapter) -> None:
    """Pure data object (string values) must still be flagged."""
    entries = [f"'key_{i}': 'value_{i}'" for i in range(50)]
    src = "export const MAP = {\n  " + ",\n  ".join(entries) + ",\n};\n"
    assert adapter.is_data_dominant(src) is True


def test_data_dominant_nested_data_array_of_objects_of_strings_ts(
    adapter: TypeScriptAdapter,
) -> None:
    """faker-js locale-style nested structure (array of objects with string values) must flag."""
    rows = [
        "{ firstName: 'Alice', lastName: 'Smith', city: 'Paris' }" for _ in range(30)
    ]
    src = "export const localeData = [\n  " + ",\n  ".join(rows) + ",\n];\n"
    assert adapter.is_data_dominant(src) is True
