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
        # `benchmarks` is conventionally non-production code (the same category
        # as `tests`/`__tests__`), and on argot itself the working-tree walk
        # would otherwise pull in gitignored bench corpus clones under
        # `benchmarks/data/.repo/`. Skipping by basename keeps the corpus
        # focused on real product code.
        "benchmarks",
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
        description="Collect repo corpus source files and BPE generic baseline for argot scoring"
    )
    parser.add_argument("--repo", default=".", help="Path to the target repository")
    parser.add_argument(
        "--repo-corpus-out",
        default=".argot/repo-corpus.txt",
        help="Output file listing repo corpus source paths",
    )
    parser.add_argument(
        "--generic-baseline-out",
        default=".argot/generic-baseline.json",
        help="Output path for the BPE generic baseline JSON",
    )
    args = parser.parse_args()

    repo_path = Path(args.repo).resolve()
    if not (repo_path / ".git").exists():
        print(f"error: not a git repository: {repo_path}", file=sys.stderr)
        sys.exit(2)

    repo_corpus_out = Path(args.repo_corpus_out)
    generic_baseline_out = Path(args.generic_baseline_out)
    repo_corpus_out.parent.mkdir(parents=True, exist_ok=True)
    generic_baseline_out.parent.mkdir(parents=True, exist_ok=True)

    files = _collect_source_files(repo_path)
    if not files:
        print("error: no source files found in repository", file=sys.stderr)
        sys.exit(2)

    repo_corpus_out.write_text("\n".join(str(p) for p in files))
    print(f"repo corpus: {len(files)} source files → {repo_corpus_out}")

    bpe_ref = Path(__file__).parent / "scoring" / "bpe" / "generic_tokens_bpe.json"
    shutil.copy(bpe_ref, generic_baseline_out)
    print(f"generic baseline: {generic_baseline_out}")


if __name__ == "__main__":
    main()
