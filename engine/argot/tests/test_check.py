from __future__ import annotations

from pathlib import Path
from typing import cast

import pygit2
import pytest

from argot.check import (
    _apply_filters,
    _highlight_lines,
    _Hit,
    _modified_patches,
    _render_results,
    _severity,
    _staged_patches,
    _untracked_patches,
)
from argot.git_walk import _resolve_shas

# ---------------------------------------------------------------------------
# Repo helpers
# ---------------------------------------------------------------------------


def _make_repo(tmp_path: Path, files: dict[str, str]) -> pygit2.Repository:
    """Create a repo with a single commit containing the given files."""
    repo = pygit2.init_repository(str(tmp_path))
    sig = pygit2.Signature("Test", "test@example.com")
    for name, content in files.items():
        (tmp_path / name).write_text(content)
        repo.index.add(name)
    repo.index.write()
    tree = repo.index.write_tree()
    repo.create_commit("refs/heads/main", sig, sig, "init", tree, [])
    repo.set_head("refs/heads/main")
    return repo


def _make_two_commit_repo(tmp_path: Path) -> pygit2.Repository:
    """Create a repo with two commits modifying main.py."""
    repo = pygit2.init_repository(str(tmp_path))
    sig = pygit2.Signature("Test", "test@example.com")
    f = tmp_path / "main.py"
    f.write_text("x = 1\n")
    repo.index.add("main.py")
    repo.index.write()
    tree1 = repo.index.write_tree()
    c1 = repo.create_commit("refs/heads/main", sig, sig, "first", tree1, [])
    f.write_text("x = 2\n")
    repo.index.add("main.py")
    repo.index.write()
    tree2 = repo.index.write_tree()
    repo.create_commit("refs/heads/main", sig, sig, "second", tree2, [c1])
    return repo


# ---------------------------------------------------------------------------
# _resolve_shas (now lives in git_walk)
# ---------------------------------------------------------------------------


def test_resolve_shas_range(tmp_path: Path) -> None:
    repo = _make_two_commit_repo(tmp_path)
    head_oid = repo.references["refs/heads/main"].target
    commit = cast(pygit2.Commit, repo.get(head_oid))
    parent_oid = commit.parents[0].id

    shas = _resolve_shas(repo, f"{parent_oid}..refs/heads/main")
    assert str(head_oid) in shas
    assert str(parent_oid) not in shas


def test_resolve_shas_bare_ref(tmp_path: Path) -> None:
    repo = _make_two_commit_repo(tmp_path)
    head_oid = str(repo.references["refs/heads/main"].target)
    shas = _resolve_shas(repo, "refs/heads/main")
    assert head_oid in shas


# ---------------------------------------------------------------------------
# _modified_patches (unstaged changes vs index)
# ---------------------------------------------------------------------------


def test_modified_patches_detects_modification(tmp_path: Path) -> None:
    """Unstaged change to a supported file is picked up."""
    _make_repo(tmp_path, {"main.py": "x = 1\n"})
    (tmp_path / "main.py").write_text("x = 1\ny = 2\n")

    patches = list(_modified_patches(str(tmp_path)))
    assert len(patches) == 1
    batch = patches[0]
    assert batch.file_path == "main.py"
    assert b"y = 2" in batch.content
    assert len(batch.hunks) > 0
    assert batch.source == "workdir"


def test_modified_patches_ignores_unsupported_extension(tmp_path: Path) -> None:
    _make_repo(tmp_path, {"config.json": "{}\n"})
    (tmp_path / "config.json").write_text('{"key": "value"}\n')

    patches = list(_modified_patches(str(tmp_path)))
    assert patches == []


def test_modified_patches_ignores_deleted_files(tmp_path: Path) -> None:
    _make_repo(tmp_path, {"main.py": "x = 1\n"})
    (tmp_path / "main.py").unlink()

    patches = list(_modified_patches(str(tmp_path)))
    assert patches == []


def test_modified_patches_no_changes(tmp_path: Path) -> None:
    _make_repo(tmp_path, {"main.py": "x = 1\n"})

    patches = list(_modified_patches(str(tmp_path)))
    assert patches == []


def test_modified_patches_ignores_staged_only(tmp_path: Path) -> None:
    """A staged-only change is NOT returned by _modified_patches."""
    repo = _make_repo(tmp_path, {"main.py": "x = 1\n"})
    # Stage a change without touching the workdir.
    (tmp_path / "main.py").write_text("x = 1\ny = 2\n")
    repo.index.add("main.py")
    repo.index.write()
    # Working directory is now in sync with the index, so no unstaged diff.
    patches = list(_modified_patches(str(tmp_path)))
    assert patches == []


# ---------------------------------------------------------------------------
# _staged_patches
# ---------------------------------------------------------------------------


def test_staged_patches_detects_staged_change(tmp_path: Path) -> None:
    """A staged change is picked up by _staged_patches."""
    repo = _make_repo(tmp_path, {"main.py": "x = 1\n"})
    (tmp_path / "main.py").write_text("x = 1\ny = 2\n")
    repo.index.add("main.py")
    repo.index.write()

    patches = list(_staged_patches(str(tmp_path)))
    assert len(patches) == 1
    batch = patches[0]
    assert batch.file_path == "main.py"
    assert b"y = 2" in batch.content
    assert len(batch.hunks) > 0
    assert batch.source == "staged"


def test_staged_patches_ignores_unsupported_extension(tmp_path: Path) -> None:
    repo = _make_repo(tmp_path, {"config.json": "{}\n"})
    (tmp_path / "config.json").write_text('{"key": "value"}\n')
    repo.index.add("config.json")
    repo.index.write()

    patches = list(_staged_patches(str(tmp_path)))
    assert patches == []


def test_staged_patches_no_staged_changes(tmp_path: Path) -> None:
    _make_repo(tmp_path, {"main.py": "x = 1\n"})
    patches = list(_staged_patches(str(tmp_path)))
    assert patches == []


# ---------------------------------------------------------------------------
# _untracked_patches
# ---------------------------------------------------------------------------


def test_untracked_patches_detects_new_file(tmp_path: Path) -> None:
    """An untracked supported file yields a full-file hunk."""
    _make_repo(tmp_path, {"existing.py": "x = 1\n"})
    (tmp_path / "new.py").write_text("a = 1\nb = 2\nc = 3\n")

    patches = list(_untracked_patches(str(tmp_path)))
    assert len(patches) == 1
    batch = patches[0]
    assert batch.file_path == "new.py"
    assert b"a = 1" in batch.content
    assert len(batch.hunks) == 1
    hunk = batch.hunks[0]
    assert hunk.new_start == 1
    assert hunk.new_lines == 3  # 3 lines in the file
    assert batch.source == "untracked"


def test_untracked_patches_skips_unsupported_extension(tmp_path: Path) -> None:
    _make_repo(tmp_path, {"existing.py": "x = 1\n"})
    (tmp_path / "data.json").write_text('{"key": "value"}\n')

    patches = list(_untracked_patches(str(tmp_path)))
    assert patches == []


def test_untracked_patches_skips_already_tracked(tmp_path: Path) -> None:
    """A tracked-but-modified file is NOT returned by _untracked_patches."""
    _make_repo(tmp_path, {"main.py": "x = 1\n"})
    (tmp_path / "main.py").write_text("x = 2\n")

    patches = list(_untracked_patches(str(tmp_path)))
    assert patches == []


# ---------------------------------------------------------------------------
# _apply_filters
# ---------------------------------------------------------------------------


def test_apply_filters_no_filters() -> None:
    paths = ["src/foo.py", "src/bar.ts", "tests/test_foo.py"]
    assert _apply_filters(paths, only=[], exclude=[]) == paths


def test_apply_filters_only() -> None:
    paths = ["src/foo.py", "src/bar.ts", "tests/test_foo.py"]
    result = _apply_filters(paths, only=["src/*.py"], exclude=[])
    assert result == ["src/foo.py"]


def test_apply_filters_exclude() -> None:
    paths = ["src/foo.py", "src/bar.ts", "tests/test_foo.py"]
    result = _apply_filters(paths, only=[], exclude=["tests/*"])
    assert result == ["src/foo.py", "src/bar.ts"]


def test_apply_filters_exclude_wins_over_only() -> None:
    """A file matching both --only and --exclude is excluded."""
    paths = ["src/foo.py"]
    result = _apply_filters(paths, only=["src/*.py"], exclude=["src/foo.py"])
    assert result == []


def test_apply_filters_multiple_only_globs() -> None:
    paths = ["src/foo.py", "src/bar.ts", "config.json"]
    result = _apply_filters(paths, only=["*.py", "*.ts"], exclude=[])
    assert result == ["src/foo.py", "src/bar.ts"]


# ---------------------------------------------------------------------------
# _severity
# ---------------------------------------------------------------------------


def test_severity_unusual() -> None:
    assert _severity(4.0, 4.0) == "unusual"
    assert _severity(4.49, 4.0) == "unusual"


def test_severity_suspicious() -> None:
    assert _severity(4.5, 4.0) == "suspicious"
    assert _severity(5.49, 4.0) == "suspicious"


def test_severity_foreign() -> None:
    assert _severity(5.5, 4.0) == "foreign"
    assert _severity(10.0, 4.0) == "foreign"


# ---------------------------------------------------------------------------
# _render_results output format
# ---------------------------------------------------------------------------


def _hit(
    score: float,
    file_path: str,
    line: int,
    *,
    line_end: int | None = None,
    source: str = "workdir",
    reason: str = "bpe",
    hunk_content: str = "",
) -> _Hit:
    """Test helper: build a _Hit with sensible defaults."""
    return _Hit(
        score=score,
        file_path=file_path,
        line=line,
        line_end=line if line_end is None else line_end,
        source=source,
        reason=reason,
        hunk_content=hunk_content,
    )


def test_render_results_banner_and_grouping(capsys: pytest.CaptureFixture[str]) -> None:
    """Banner shows tier counts; hits are grouped by file with correct format."""
    threshold = 4.0
    hits = [
        _hit(6.0, "src/foo.py", 10),  # foreign
        _hit(4.6, "src/foo.py", 5),  # suspicious
        _hit(4.1, "src/bar.ts", 20),  # unusual
    ]
    _render_results(hits, threshold, use_color=False, hunk_lines=0)
    out = capsys.readouterr().out

    assert "argot check" in out
    assert "3 hunks above threshold" in out
    assert "1 foreign" in out
    assert "1 suspicious" in out
    assert "1 unusual" in out
    assert "note: argot is a probabilistic style linter" in out

    # Files should appear; foo.py first (max score 6.0 > 4.1)
    foo_pos = out.index("src/foo.py")
    bar_pos = out.index("src/bar.ts")
    assert foo_pos < bar_pos

    # Within foo.py, line 5 before line 10 (asc sort)
    assert out.index("L5") < out.index("L10")

    # Severity labels present
    assert "foreign" in out
    assert "suspicious" in out
    assert "unusual" in out

    # ASCII glyphs used when use_color=False
    assert "!" in out
    assert "?" in out
    assert "." in out


def test_render_results_zero_tiers_omitted(capsys: pytest.CaptureFixture[str]) -> None:
    """Tier counts with 0 hits are not shown in the banner."""
    threshold = 4.0
    hits = [_hit(6.0, "src/foo.py", 1)]  # foreign only
    _render_results(hits, threshold, use_color=False, hunk_lines=0)
    out = capsys.readouterr().out
    assert "foreign" in out
    assert "0 suspicious" not in out
    assert "0 unusual" not in out


def test_render_results_singular_hunk(capsys: pytest.CaptureFixture[str]) -> None:
    """Banner says '1 hunk' not '1 hunks'."""
    threshold = 4.0
    hits = [_hit(6.0, "src/foo.py", 1)]
    _render_results(hits, threshold, use_color=False, hunk_lines=0)
    out = capsys.readouterr().out
    assert "1 hunk above threshold" in out
    assert "1 hunks" not in out


def test_render_results_shows_source_and_reason(capsys: pytest.CaptureFixture[str]) -> None:
    """Source label and friendly scorer reason appear on each headline."""
    threshold = 4.0
    hits = [
        _hit(6.0, "src/foo.py", 1, source="cac6278", reason="bpe"),
        _hit(5.5, "src/bar.py", 2, source="staged", reason="call_receiver"),
    ]
    _render_results(hits, threshold, use_color=False, hunk_lines=0)
    out = capsys.readouterr().out
    assert "cac6278" in out
    assert "staged" in out
    # Friendly labels visible; raw codes preserved in parens for traceability.
    assert "rare token sequence" in out
    assert "(bpe)" in out
    assert "unfamiliar callee" in out
    assert "(call_receiver)" in out


def test_render_results_shows_line_range(capsys: pytest.CaptureFixture[str]) -> None:
    """Multi-line hunks show start–end range; single-line hunks show one number."""
    threshold = 4.0
    hits = [
        _hit(6.0, "src/foo.py", 10, line_end=15),  # multi-line
        _hit(5.0, "src/bar.py", 42),  # single-line (line_end == line)
    ]
    _render_results(hits, threshold, use_color=False, hunk_lines=0)
    out = capsys.readouterr().out
    assert "L10-L15" in out
    assert "L42" in out
    assert "L42-" not in out  # single-line hunk uses bare form, no dash


def test_render_results_disambiguates_dupes_by_source(
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Two same-line same-score hits in different commits stay distinguishable."""
    threshold = 4.0
    hits = [
        _hit(5.65, "scripts/markdown.ts", 39, source="cac6278", reason="bpe"),
        _hit(5.65, "scripts/markdown.ts", 39, source="325e402", reason="bpe"),
    ]
    _render_results(hits, threshold, use_color=False, hunk_lines=0)
    out = capsys.readouterr().out
    # Both rows render, each labeled with its commit SHA — duplicates stop
    # looking like duplicates.
    assert "cac6278" in out
    assert "325e402" in out


def test_render_results_shows_hunk_content(capsys: pytest.CaptureFixture[str]) -> None:
    """Hunk body lines render with line numbers under each headline up to hunk_lines."""
    threshold = 4.0
    body = "line one\nline two\nline three\nline four\nline five"
    hits = [_hit(6.0, "src/foo.py", 100, line_end=104, hunk_content=body)]
    _render_results(hits, threshold, use_color=False, hunk_lines=3)
    out = capsys.readouterr().out
    assert "line one" in out
    assert "line two" in out
    assert "line three" in out
    assert "line four" not in out  # truncated
    assert "(+2 more lines)" in out
    # Line numbers appear in the gutter (start_line was 100).
    assert "100" in out
    assert "101" in out
    assert "102" in out


def test_render_results_hunk_lines_zero_suppresses_body(
    capsys: pytest.CaptureFixture[str],
) -> None:
    """hunk_lines=0 hides the hunk body entirely."""
    threshold = 4.0
    body = "secret content\nshould not appear"
    hits = [_hit(6.0, "src/foo.py", 1, hunk_content=body)]
    _render_results(hits, threshold, use_color=False, hunk_lines=0)
    out = capsys.readouterr().out
    assert "secret content" not in out
    assert "should not appear" not in out


# ---------------------------------------------------------------------------
# _highlight_lines — pygments alignment
# ---------------------------------------------------------------------------


def test_highlight_lines_pads_leading_empty_line() -> None:
    """Pygments collapses leading empty lines; we re-pad so counts match.

    Regression: when the hunk's first line was empty, the renderer indexed
    past the end of the highlighted list (IndexError, exit code 1) and the
    bug was masked by the CLI adapter treating exit 1 as 'has violations'.
    """
    content = "\n      expect(actual).toBe(true);\n    });"
    raw = content.splitlines()
    # use_color=True triggers the pygments path that collapses leading "".
    highlighted = _highlight_lines(content, "test.ts", use_color=True)
    assert len(highlighted) == len(
        raw
    ), f"highlighted/raw mismatch: {len(highlighted)} vs {len(raw)}"


def test_highlight_lines_no_color_returns_raw_lines() -> None:
    """Without colors, returns the source lines verbatim (no pygments)."""
    content = "\nline two\nline three"
    raw = content.splitlines()
    assert _highlight_lines(content, "test.ts", use_color=False) == raw


def test_render_results_verbose_hunk_starting_with_empty_line(
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Full body renders for a hunk whose first line is empty (regression).

    Exercises the renderer with use_color=True + hunk_lines=None (verbose),
    where the IndexError originally killed the run mid-hit. Asserts every
    source line shows up in the output.
    """
    threshold = 4.0
    # First line empty — the exact shape that exposed the bug.
    body = "\n      expect(x).toBe(true);\n    });\n  });"
    hits = [_hit(6.0, "src/foo.ts", 46, line_end=49, hunk_content=body)]
    # use_color=True is the path that hits pygments.
    _render_results(hits, threshold, use_color=True, hunk_lines=None)
    out = capsys.readouterr().out
    # Identifier-only fragments survive pygments token-level coloring; full
    # phrases like "toBe(true)" wouldn't because keywords get wrapped in
    # ANSI escapes that break the substring match.
    assert "expect(x)" in out
    assert "toBe(" in out
    # Line numbers cover the full range, not just the first.
    assert "46" in out
    assert "47" in out
    assert "48" in out
    assert "49" in out
