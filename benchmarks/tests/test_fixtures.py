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
