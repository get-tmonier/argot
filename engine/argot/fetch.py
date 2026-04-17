from __future__ import annotations

import subprocess
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path

from argot.extract import main as extract_main


@dataclass(frozen=True)
class RepoConfig:
    name: str
    url: str
    limit: int
    label: str


def load_repos_config(config_path: Path) -> list[RepoConfig]:
    data = tomllib.loads(config_path.read_text())
    return [RepoConfig(**r) for r in data["repos"]]


def merge_jsonl(sources: list[tuple[Path, str]], dest: Path) -> int:
    import json

    count = 0
    with dest.open("w") as fh:
        for src, repo_name in sources:
            for line in src.read_text().splitlines():
                if line.strip():
                    record = json.loads(line)
                    record["_repo"] = repo_name
                    fh.write(json.dumps(record) + "\n")
                    count += 1
    return count


def _clone_or_update(url: str, dest: Path, depth: int = 2000) -> None:
    if (dest / ".git").exists():
        print(f"  already cloned, skipping: {dest}")
        return
    dest.parent.mkdir(parents=True, exist_ok=True)
    print(f"  cloning {url} → {dest}")
    subprocess.run(
        ["git", "clone", "--depth", str(depth), url, str(dest)],
        check=True,
    )


def main() -> None:
    project_root = Path(__file__).parents[2]  # engine/argot/fetch.py → project root
    config_path = project_root / "training" / "repos.toml"

    if not config_path.exists():
        print(f"error: config not found at {config_path}", file=sys.stderr)
        sys.exit(2)

    repos = load_repos_config(config_path)
    repos_dir = project_root / ".argot" / "repos"
    part_files: list[tuple[Path, str]] = []

    for repo in repos:
        repo_dir = repos_dir / repo.name
        part_out = repos_dir / f"{repo.name}.jsonl"
        print(f"\n[{repo.name}] label={repo.label} limit={repo.limit}")
        _clone_or_update(repo.url, repo_dir)

        sys.argv = [
            "extract",
            str(repo_dir),
            "--out",
            str(part_out),
            "--limit",
            str(repo.limit),
        ]
        extract_main()
        part_files.append((part_out, repo.name))

    out = project_root / ".argot" / "training.jsonl"
    total = merge_jsonl(part_files, out)
    print(f"\nMerged {total} records → {out}")


if __name__ == "__main__":
    main()
