from pathlib import Path

import pytest

from argot_bench.fixtures import load_catalog, scan_all_catalogs

CATALOGS_DIR = Path(__file__).parent.parent / "catalogs"


def test_scan_all_catalogs_finds_pinned_corpora():
    names = sorted(c.corpus for c in scan_all_catalogs(CATALOGS_DIR))
    assert names == [
        "dagster",
        "faker",
        "faker-js",
        "fastapi",
        "hono",
        "ink",
        "rich",
    ]


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


def test_rich_has_sixteen_fixtures():
    cat = load_catalog(CATALOGS_DIR / "rich")
    assert len(cat.fixtures) == 16, f"Expected 16, got {len(cat.fixtures)}"


def test_rich_each_category_has_three_fixtures():
    cat = load_catalog(CATALOGS_DIR / "rich")
    by_cat: dict[str, int] = {}
    for fx in cat.fixtures:
        by_cat[fx.category] = by_cat.get(fx.category, 0) + 1
    for cat_name, count in by_cat.items():
        assert count >= 3, f"rich category {cat_name!r} has only {count} fixtures (need >=3)"


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


def test_all_existing_fixtures_have_difficulty_label():
    """Every fixture in every catalog must have a non-None difficulty label."""
    cats = scan_all_catalogs(CATALOGS_DIR)
    missing = []
    for cat in cats:
        for fx in cat.fixtures:
            if fx.difficulty is None:
                missing.append(f"{cat.corpus}:{fx.id}")
    assert not missing, f"Fixtures missing difficulty label: {missing}"


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


def test_faker_has_sixteen_fixtures():
    cat = load_catalog(CATALOGS_DIR / "faker")
    assert len(cat.fixtures) == 16, f"Expected 16, got {len(cat.fixtures)}"


def test_faker_each_category_has_three_fixtures():
    cat = load_catalog(CATALOGS_DIR / "faker")
    by_cat: dict[str, int] = {}
    for fx in cat.fixtures:
        by_cat[fx.category] = by_cat.get(fx.category, 0) + 1
    for cat_name, count in by_cat.items():
        assert count >= 3, f"faker category {cat_name!r} has only {count} fixtures (need >=3)"


def test_catalog_structural_requirements():
    """Every corpus has enough fixtures and categories for stable metrics.

    Single-language: >=15 fixtures, >=5 categories, >=3 fixtures per
    category. Multi-language: each language sub-corpus needs >=10
    fixtures, >=4 categories, >=2 fixtures per (language, category).
    The relaxed multi-language bounds reflect that the catalog's total
    fixture budget is split across two languages — per-category recall
    stability is a function of *within-language* count, not the global
    sum that conflates Python and TypeScript categories.
    """
    cats = scan_all_catalogs(CATALOGS_DIR)
    failures: list[str] = []
    for cat in cats:
        if cat.language == "multi":
            by_lang: dict[str, int] = {}
            by_lang_cat: dict[tuple[str, str], int] = {}
            for fx in cat.fixtures:
                lang = fx.language
                assert lang is not None, f"{cat.corpus}:{fx.id} missing language"
                by_lang[lang] = by_lang.get(lang, 0) + 1
                by_lang_cat[(lang, fx.category)] = by_lang_cat.get((lang, fx.category), 0) + 1
            for lang, n_fix in by_lang.items():
                if n_fix < 10:
                    failures.append(f"{cat.corpus}/{lang}: {n_fix} fixtures (need >=10)")
                lang_cat_count = sum(1 for (lang_key, _) in by_lang_cat if lang_key == lang)
                if lang_cat_count < 4:
                    failures.append(
                        f"{cat.corpus}/{lang}: {lang_cat_count} categories (need >=4)"
                    )
            for (lang, cat_name), count in by_lang_cat.items():
                if count < 2:
                    failures.append(
                        f"{cat.corpus}/{lang}/{cat_name}: {count} fixtures (need >=2)"
                    )
        else:
            n_fix = len(cat.fixtures)
            n_cats = len(cat.categories)
            if n_fix < 15:
                failures.append(f"{cat.corpus}: {n_fix} fixtures (need >=15)")
            if n_cats < 5:
                failures.append(f"{cat.corpus}: {n_cats} categories (need >=5)")
            by_cat: dict[str, int] = {}
            for fx in cat.fixtures:
                by_cat[fx.category] = by_cat.get(fx.category, 0) + 1
            for cat_name, count in by_cat.items():
                if count < 3:
                    failures.append(f"{cat.corpus}/{cat_name}: {count} fixtures (need >=3)")
    assert not failures, "\n".join(failures)


def test_era7_difficulty_coverage_all_fixtures():
    """Gate 4: every fixture (old + new) has a difficulty label."""
    cats = scan_all_catalogs(CATALOGS_DIR)
    missing = []
    for cat in cats:
        for fx in cat.fixtures:
            if fx.difficulty is None:
                missing.append(f"{cat.corpus}:{fx.id}")
    assert not missing, f"Fixtures missing difficulty label: {missing}"


# ---------------------------------------------------------------------------
# Era-14 Fix A — host_file injection schema
# ---------------------------------------------------------------------------


def test_fixture_host_fields_default_to_none():
    """Without host_file/host_inject_at_line set, both default to None."""
    from argot_bench.fixtures import Fixture
    fx = Fixture(
        id="x", file="f.py", category="c",
        hunk_start_line=1, hunk_end_line=5,
    )
    assert fx.host_file is None
    assert fx.host_inject_at_line is None


def test_fixture_loads_host_file_fields(tmp_path: Path):
    """A YAML manifest with host_file/host_inject_at_line parses correctly."""
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
        "    host_file: src/host.py\n"
        "    host_inject_at_line: 42\n"
    )
    cat = load_catalog(catalog_dir)
    assert cat.fixtures[0].host_file == "src/host.py"
    assert cat.fixtures[0].host_inject_at_line == 42


def test_fixture_validates_host_fields_paired_only_host(tmp_path: Path):
    """host_file alone (no host_inject_at_line) raises a clear error."""
    catalog_dir = tmp_path / "mycorpus"
    catalog_dir.mkdir()
    breaks_dir = catalog_dir / "breaks"
    breaks_dir.mkdir()
    (breaks_dir / "break_test.py").write_text("\n".join(["# line"] * 10))
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
        "    difficulty: medium\n"
        "    host_file: src/host.py\n"
    )
    with pytest.raises(ValueError, match="host_file and host_inject_at_line"):
        load_catalog(catalog_dir)


def test_fixture_validates_host_fields_paired_only_line(tmp_path: Path):
    """host_inject_at_line alone (no host_file) raises a clear error."""
    catalog_dir = tmp_path / "mycorpus"
    catalog_dir.mkdir()
    breaks_dir = catalog_dir / "breaks"
    breaks_dir.mkdir()
    (breaks_dir / "break_test.py").write_text("\n".join(["# line"] * 10))
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
        "    difficulty: medium\n"
        "    host_inject_at_line: 10\n"
    )
    with pytest.raises(ValueError, match="host_file and host_inject_at_line"):
        load_catalog(catalog_dir)


def test_fixture_host_inject_at_line_must_be_positive(tmp_path: Path):
    """host_inject_at_line < 1 raises a clear error."""
    catalog_dir = tmp_path / "mycorpus"
    catalog_dir.mkdir()
    breaks_dir = catalog_dir / "breaks"
    breaks_dir.mkdir()
    (breaks_dir / "break_test.py").write_text("\n".join(["# line"] * 10))
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
        "    difficulty: medium\n"
        "    host_file: src/host.py\n"
        "    host_inject_at_line: 0\n"
    )
    with pytest.raises(ValueError, match="host_inject_at_line must be >= 1"):
        load_catalog(catalog_dir)


def test_fixture_without_host_fields_loads_unchanged(tmp_path: Path):
    """A fixture without host_file/host_inject_at_line still loads (backward compat)."""
    catalog_dir = tmp_path / "mycorpus"
    catalog_dir.mkdir()
    breaks_dir = catalog_dir / "breaks"
    breaks_dir.mkdir()
    (breaks_dir / "break_test.py").write_text("\n".join(["# line"] * 10))
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
        "    difficulty: medium\n"
    )
    cat = load_catalog(catalog_dir)
    assert cat.fixtures[0].host_file is None
    assert cat.fixtures[0].host_inject_at_line is None


# ---------------------------------------------------------------------------
# Per-fixture language field (multi-language catalog support)
# ---------------------------------------------------------------------------


def test_fixture_language_defaults_to_none():
    """Fixture.language defaults to None when not specified."""
    from argot_bench.fixtures import Fixture
    fx = Fixture(id="x", file="f.py", category="c", hunk_start_line=1, hunk_end_line=5)
    assert fx.language is None


def test_fixture_language_explicit_python(tmp_path: Path):
    """A fixture with language: python is parsed correctly."""
    catalog_dir = tmp_path / "corp"
    catalog_dir.mkdir()
    (catalog_dir / "breaks").mkdir()
    (catalog_dir / "breaks" / "b.py").write_text("# line\n" * 10)
    (catalog_dir / "manifest.yaml").write_text(
        "corpus: corp\n"
        "language: python\n"
        "categories:\n  - cat_a\n"
        "injection_hosts: []\n"
        "fixtures:\n"
        "  - id: f1\n"
        "    file: breaks/b.py\n"
        "    category: cat_a\n"
        "    hunk_start_line: 1\n"
        "    hunk_end_line: 5\n"
        "    difficulty: easy\n"
        "    language: python\n"
    )
    cat = load_catalog(catalog_dir)
    assert cat.fixtures[0].language == "python"


def test_fixture_language_explicit_typescript(tmp_path: Path):
    """A fixture with language: typescript is parsed correctly."""
    catalog_dir = tmp_path / "corp"
    catalog_dir.mkdir()
    (catalog_dir / "breaks").mkdir()
    (catalog_dir / "breaks" / "b.ts").write_text("// line\n" * 10)
    (catalog_dir / "manifest.yaml").write_text(
        "corpus: corp\n"
        "language: typescript\n"
        "categories:\n  - cat_a\n"
        "injection_hosts: []\n"
        "fixtures:\n"
        "  - id: f1\n"
        "    file: breaks/b.ts\n"
        "    category: cat_a\n"
        "    hunk_start_line: 1\n"
        "    hunk_end_line: 5\n"
        "    difficulty: easy\n"
        "    language: typescript\n"
    )
    cat = load_catalog(catalog_dir)
    assert cat.fixtures[0].language == "typescript"


def test_fixture_language_invalid_value_raises(tmp_path: Path):
    """An unrecognised fixture language raises a clear error."""
    catalog_dir = tmp_path / "corp"
    catalog_dir.mkdir()
    (catalog_dir / "breaks").mkdir()
    (catalog_dir / "breaks" / "b.py").write_text("# line\n" * 10)
    (catalog_dir / "manifest.yaml").write_text(
        "corpus: corp\n"
        "language: python\n"
        "categories:\n  - cat_a\n"
        "injection_hosts: []\n"
        "fixtures:\n"
        "  - id: f1\n"
        "    file: breaks/b.py\n"
        "    category: cat_a\n"
        "    hunk_start_line: 1\n"
        "    hunk_end_line: 5\n"
        "    difficulty: easy\n"
        "    language: ruby\n"
    )
    with pytest.raises(ValueError, match="invalid fixture language"):
        load_catalog(catalog_dir)


def test_fixture_language_absent_in_single_language_catalog(tmp_path: Path):
    """Single-language catalog fixtures may omit per-fixture language (backward compat)."""
    catalog_dir = tmp_path / "corp"
    catalog_dir.mkdir()
    (catalog_dir / "breaks").mkdir()
    (catalog_dir / "breaks" / "b.py").write_text("# line\n" * 10)
    (catalog_dir / "manifest.yaml").write_text(
        "corpus: corp\n"
        "language: python\n"
        "categories:\n  - cat_a\n"
        "injection_hosts: []\n"
        "fixtures:\n"
        "  - id: f1\n"
        "    file: breaks/b.py\n"
        "    category: cat_a\n"
        "    hunk_start_line: 1\n"
        "    hunk_end_line: 5\n"
        "    difficulty: easy\n"
    )
    cat = load_catalog(catalog_dir)
    assert cat.fixtures[0].language is None
    assert cat.language == "python"


def test_multi_catalog_with_per_fixture_language_loads(tmp_path: Path):
    """A catalog with language: multi + per-fixture language fields loads correctly."""
    catalog_dir = tmp_path / "dagster"
    catalog_dir.mkdir()
    py_dir = catalog_dir / "breaks" / "py"
    py_dir.mkdir(parents=True)
    ts_dir = catalog_dir / "breaks" / "ts"
    ts_dir.mkdir(parents=True)
    (py_dir / "break_1.py").write_text("# line\n" * 10)
    (ts_dir / "break_1.ts").write_text("// line\n" * 10)
    (catalog_dir / "manifest.yaml").write_text(
        "corpus: dagster\n"
        "language: multi\n"
        "categories:\n  - cat_a\n"
        "injection_hosts: []\n"
        "fixtures:\n"
        "  - id: py_1\n"
        "    file: breaks/py/break_1.py\n"
        "    category: cat_a\n"
        "    hunk_start_line: 1\n"
        "    hunk_end_line: 5\n"
        "    difficulty: easy\n"
        "    language: python\n"
        "  - id: ts_1\n"
        "    file: breaks/ts/break_1.ts\n"
        "    category: cat_a\n"
        "    hunk_start_line: 1\n"
        "    hunk_end_line: 5\n"
        "    difficulty: easy\n"
        "    language: typescript\n"
    )
    cat = load_catalog(catalog_dir)
    assert cat.language == "multi"
    assert cat.fixtures[0].language == "python"
    assert cat.fixtures[1].language == "typescript"


def test_multi_catalog_fixture_missing_language_raises(tmp_path: Path):
    """Multi catalog with a fixture missing per-fixture language raises a clear error."""
    catalog_dir = tmp_path / "dagster"
    catalog_dir.mkdir()
    (catalog_dir / "breaks").mkdir()
    (catalog_dir / "breaks" / "b.py").write_text("# line\n" * 10)
    (catalog_dir / "manifest.yaml").write_text(
        "corpus: dagster\n"
        "language: multi\n"
        "categories:\n  - cat_a\n"
        "injection_hosts: []\n"
        "fixtures:\n"
        "  - id: f1\n"
        "    file: breaks/b.py\n"
        "    category: cat_a\n"
        "    hunk_start_line: 1\n"
        "    hunk_end_line: 5\n"
        "    difficulty: easy\n"
    )
    with pytest.raises(ValueError, match="fixtures missing per-fixture language"):
        load_catalog(catalog_dir)
