from __future__ import annotations

from pathlib import Path

from argot.scoring.adapters.language_adapter import LanguageAdapter
from argot.scoring.adapters.typescript import TypeScriptAdapter

_CATALOG = Path(__file__).parent.parent / "acceptance" / "catalog"
_KY_FIXTURES = _CATALOG / "ky" / "fixtures" / "default"

_TS_FILES = sorted(_KY_FIXTURES.glob("*.ts"))[:3]


def _read(p: Path) -> str:
    return p.read_text(encoding="utf-8", errors="replace")


def test_typescript_adapter_implements_protocol() -> None:
    adapter = TypeScriptAdapter()
    assert isinstance(adapter, LanguageAdapter)


def test_file_extensions_includes_ts() -> None:
    adapter = TypeScriptAdapter()
    assert ".ts" in adapter.file_extensions
    assert ".tsx" in adapter.file_extensions


def test_enumerate_sampleable_ranges_on_ky_fixtures() -> None:
    """enumerate_sampleable_ranges returns non-empty list for TS fixture files."""
    adapter = TypeScriptAdapter()
    for f in _TS_FILES:
        src = _read(f)
        ranges = adapter.enumerate_sampleable_ranges(src)
        assert isinstance(ranges, list), f"{f.name}: expected list"
        assert len(ranges) > 0, f"{f.name}: no sampleable ranges found"
        for start, end in ranges:
            assert isinstance(start, int)
            assert isinstance(end, int)
            assert start < end, f"{f.name}: start {start} >= end {end}"


def test_is_data_dominant_false_for_control_files() -> None:
    """ky control files contain logic, not data literals."""
    adapter = TypeScriptAdapter()
    for f in (_KY_FIXTURES / ".." / "..").parent.glob("**/control_*.ts"):
        src = _read(f)
        assert not adapter.is_data_dominant(src), f"{f.name} should not be data-dominant"
        break  # Just verify the first one found


def test_is_data_dominant_true_for_inline_data_ts() -> None:
    """A TypeScript file that is pure data export is data-dominant."""
    adapter = TypeScriptAdapter()
    data_ts = (
        "export default {\n"
        "  en: 'English',\n"
        "  fr: 'French',\n"
        "  de: 'German',\n"
        "  es: 'Spanish',\n"
        "  it: 'Italian',\n"
        "  pt: 'Portuguese',\n"
        "  nl: 'Dutch',\n"
        "  pl: 'Polish',\n"
        "  ru: 'Russian',\n"
        "  zh: 'Chinese',\n"
        "  ja: 'Japanese',\n"
        "  ko: 'Korean',\n"
        "  ar: 'Arabic',\n"
        "  hi: 'Hindi',\n"
        "  sv: 'Swedish',\n"
        "  da: 'Danish',\n"
        "  fi: 'Finnish',\n"
        "  no: 'Norwegian',\n"
        "  tr: 'Turkish',\n"
        "  uk: 'Ukrainian',\n"
        "} as const;\n"
    )
    assert adapter.is_data_dominant(data_ts)


def test_extract_imports_returns_set_of_strings_for_ts() -> None:
    adapter = TypeScriptAdapter()
    src = "import ky from 'ky';\nimport type { Options } from 'ky';\n"
    imports = adapter.extract_imports(src)
    assert isinstance(imports, set)
    assert "ky" in imports


def test_prose_line_ranges_returns_frozenset_for_ts() -> None:
    adapter = TypeScriptAdapter()
    src = "/** A JSDoc comment. */\nfunction foo() {\n  return 1;\n}\n"
    result = adapter.prose_line_ranges(src)
    assert isinstance(result, frozenset)


def test_enumerate_sampleable_ranges_inline_function() -> None:
    """A minimal TS function has at least one sampleable range."""
    adapter = TypeScriptAdapter()
    src = (
        "function greet(name: string): string {\n"
        "  const prefix = 'Hello';\n"
        "  const suffix = 'World';\n"
        "  return `${prefix}, ${name || suffix}!`;\n"
        "}\n"
    )
    ranges = adapter.enumerate_sampleable_ranges(src)
    assert len(ranges) >= 1
