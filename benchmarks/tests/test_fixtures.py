from pathlib import Path

import pytest

from argot_bench.fixtures import load_catalog, scan_all_catalogs

CATALOGS_DIR = Path(__file__).parent.parent / "catalogs"


def test_scan_all_catalogs_finds_six():
    names = sorted(c.corpus for c in scan_all_catalogs(CATALOGS_DIR))
    assert names == ["faker", "faker-js", "fastapi", "hono", "ink", "rich"]


@pytest.mark.parametrize(
    "corpus", ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
)
def test_catalog_fixtures_referenceable_files(corpus: str):
    cat = load_catalog(CATALOGS_DIR / corpus)
    assert cat.fixtures, f"{corpus} has no fixtures"
    for f in cat.fixtures:
        fpath = CATALOGS_DIR / corpus / f.file
        assert fpath.exists(), f"{corpus}:{f.id} missing file {f.file}"
        text = fpath.read_text(encoding="utf-8")
        lines = text.splitlines()
        assert f.hunk_start_line >= 1
        assert f.hunk_end_line >= f.hunk_start_line
        assert f.hunk_end_line <= len(lines), (
            f"{corpus}:{f.id} hunk_end_line {f.hunk_end_line} > file lines {len(lines)}"
        )


@pytest.mark.parametrize(
    "corpus", ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
)
def test_every_fixture_category_is_declared(corpus: str):
    cat = load_catalog(CATALOGS_DIR / corpus)
    declared = set(cat.categories)
    for f in cat.fixtures:
        assert f.category in declared, (
            f"{corpus}:{f.id} uses undeclared category {f.category!r}"
        )


def test_fixture_difficulty_defaults_to_none():
    from argot_bench.fixtures import Fixture
    fx = Fixture(id="x", file="f.py", category="c",
                 hunk_start_line=1, hunk_end_line=5)
    assert fx.difficulty is None


def test_fixture_difficulty_loaded_from_yaml(tmp_path: Path):
    catalog_dir = tmp_path / "mycorpus"
    catalog_dir.mkdir()
    breaks_dir = catalog_dir / "breaks"
    breaks_dir.mkdir()
    fx_file = breaks_dir / "break_test.py"
    fx_file.write_text("\n".join(["# line"] * 10))
    manifest = catalog_dir / "manifest.yaml"
    manifest.write_text(
        "corpus: mycorpus\n"
        "language: python\n"
        "categories:\n  - cat_a\n"
        "injection_hosts: []\n"
        "fixtures:\n"
        "  - id: test_1\n"
        "    file: breaks/break_test.py\n"
        "    category: cat_a\n"
        "    hunk_start_line: 1\n"
        "    hunk_end_line: 5\n"
        "    rationale: 'test'\n"
        "    difficulty: medium\n"
    )
    from argot_bench.fixtures import load_catalog
    cat = load_catalog(catalog_dir)
    assert cat.fixtures[0].difficulty == "medium"


def test_fixture_difficulty_optional_in_yaml(tmp_path: Path):
    catalog_dir = tmp_path / "mycorpus"
    catalog_dir.mkdir()
    breaks_dir = catalog_dir / "breaks"
    breaks_dir.mkdir()
    fx_file = breaks_dir / "break_test.py"
    fx_file.write_text("\n".join(["# line"] * 10))
    manifest = catalog_dir / "manifest.yaml"
    manifest.write_text(
        "corpus: mycorpus\n"
        "language: python\n"
        "categories:\n  - cat_a\n"
        "injection_hosts: []\n"
        "fixtures:\n"
        "  - id: test_1\n"
        "    file: breaks/break_test.py\n"
        "    category: cat_a\n"
        "    hunk_start_line: 1\n"
        "    hunk_end_line: 5\n"
    )
    from argot_bench.fixtures import load_catalog
    cat = load_catalog(catalog_dir)
    assert cat.fixtures[0].difficulty is None
