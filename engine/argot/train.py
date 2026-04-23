from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path

_SOURCE_EXTENSIONS: frozenset[str] = frozenset({".py", ".ts", ".tsx"})
_EXCLUDE_DIRS: frozenset[str] = frozenset(
    {
        "node_modules",
        ".git",
        ".tox",
        ".eggs",
        "__pycache__",
        "build",
        "dist",
        ".venv",
        "venv",
        ".mypy_cache",
        ".pytest_cache",
        ".ruff_cache",
        "test",
        "tests",
        "__tests__",
    }
)


def _collect_source_files(repo_path: Path) -> list[Path]:
    """Return all source files from repo_path, excluding test and build dirs."""
    files: list[Path] = []
    for p in repo_path.rglob("*"):
        if not p.is_file():
            continue
        if p.suffix not in _SOURCE_EXTENSIONS:
            continue
        parts = set(p.relative_to(repo_path).parts[:-1])
        if parts & _EXCLUDE_DIRS:
            continue
        name = p.name
        if name.startswith("test_") or ".test." in name or ".spec." in name:
            continue
        files.append(p)
    return files


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Collect model-A source files and BPE reference for argot scoring"
    )
    parser.add_argument("--repo", default=".", help="Path to the target repository")
    parser.add_argument(
        "--model-a-out",
        default=".argot/model_a.txt",
        help="Output file listing model-A source paths",
    )
    parser.add_argument(
        "--model-b-out",
        default=".argot/model_b.json",
        help="Output path for the BPE reference JSON",
    )
    args = parser.parse_args()

    repo_path = Path(args.repo).resolve()
    if not (repo_path / ".git").exists():
        print(f"error: not a git repository: {repo_path}", file=sys.stderr)
        sys.exit(2)

    model_a_out = Path(args.model_a_out)
    model_b_out = Path(args.model_b_out)
    model_a_out.parent.mkdir(parents=True, exist_ok=True)
    model_b_out.parent.mkdir(parents=True, exist_ok=True)

    files = _collect_source_files(repo_path)
    if not files:
        print("error: no source files found in repository", file=sys.stderr)
        sys.exit(2)

    model_a_out.write_text("\n".join(str(p) for p in files))
    print(f"model_a: {len(files)} source files → {model_a_out}")

    bpe_ref = Path(__file__).parent / "scoring" / "bpe" / "generic_tokens_bpe.json"
    shutil.copy(bpe_ref, model_b_out)
    print(f"model_b: {model_b_out}")


if __name__ == "__main__":
    main()
