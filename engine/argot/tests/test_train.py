from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pygit2

from argot.train import _collect_source_files


def _init_repo(tmp_path: Path) -> pygit2.Repository:
    repo = pygit2.init_repository(str(tmp_path))
    sig = pygit2.Signature("Test", "test@example.com")
    (tmp_path / ".gitkeep").write_text("")
    repo.index.add(".gitkeep")
    repo.index.write()
    tree = repo.index.write_tree()
    repo.create_commit("refs/heads/main", sig, sig, "init", tree, [])
    return repo


def test_collect_finds_py_ts_tsx(tmp_path: Path) -> None:
    _init_repo(tmp_path)
    (tmp_path / "main.py").write_text("x = 1\n")
    (tmp_path / "index.ts").write_text("const x = 1;\n")
    (tmp_path / "App.tsx").write_text("export default () => null;\n")
    (tmp_path / "readme.md").write_text("# docs\n")

    found = {p.name for p in _collect_source_files(tmp_path)}
    assert "main.py" in found
    assert "index.ts" in found
    assert "App.tsx" in found
    assert "readme.md" not in found


def test_collect_excludes_node_modules(tmp_path: Path) -> None:
    _init_repo(tmp_path)
    nm = tmp_path / "node_modules" / "lib"
    nm.mkdir(parents=True)
    (nm / "util.ts").write_text("export {};\n")
    (tmp_path / "src.ts").write_text("const x = 1;\n")

    found = {p.name for p in _collect_source_files(tmp_path)}
    assert "util.ts" not in found
    assert "src.ts" in found


def test_collect_excludes_test_files(tmp_path: Path) -> None:
    _init_repo(tmp_path)
    (tmp_path / "app.ts").write_text("export {};\n")
    (tmp_path / "app.test.ts").write_text("test('x', () => {});\n")
    (tmp_path / "app.spec.ts").write_text("test('x', () => {});\n")
    (tmp_path / "test_main.py").write_text("def test_x(): pass\n")

    found = {p.name for p in _collect_source_files(tmp_path)}
    assert "app.ts" in found
    assert "app.test.ts" not in found
    assert "app.spec.ts" not in found
    assert "test_main.py" not in found


def test_collect_excludes_venv_and_build_dirs(tmp_path: Path) -> None:
    _init_repo(tmp_path)
    for excluded_dir in (".venv", "dist", "__pycache__", "build"):
        d = tmp_path / excluded_dir
        d.mkdir()
        (d / "module.py").write_text("x = 1\n")
    (tmp_path / "keep.py").write_text("x = 1\n")

    found = {p.name for p in _collect_source_files(tmp_path)}
    assert "module.py" not in found
    assert "keep.py" in found


def test_collect_returns_empty_for_no_source_files(tmp_path: Path) -> None:
    _init_repo(tmp_path)
    (tmp_path / "README.md").write_text("# docs\n")
    assert _collect_source_files(tmp_path) == []


def test_main_writes_model_a_and_model_b(tmp_path: Path) -> None:
    _init_repo(tmp_path)
    (tmp_path / "main.py").write_text("x = 1\n")
    (tmp_path / "util.ts").write_text("export {};\n")

    model_a = tmp_path / "out" / "model_a.txt"
    model_b = tmp_path / "out" / "model_b.json"

    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "argot.train",
            "--repo",
            str(tmp_path),
            "--model-a-out",
            str(model_a),
            "--model-b-out",
            str(model_b),
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
    assert model_a.exists()
    assert model_b.exists()
    paths = model_a.read_text().splitlines()
    assert any("main.py" in p for p in paths)
    assert any("util.ts" in p for p in paths)


def test_main_exits_nonzero_when_no_git_repo(tmp_path: Path) -> None:
    result = subprocess.run(
        [sys.executable, "-m", "argot.train", "--repo", str(tmp_path)],
        capture_output=True,
        text=True,
    )
    assert result.returncode != 0
    assert "not a git repository" in result.stderr
