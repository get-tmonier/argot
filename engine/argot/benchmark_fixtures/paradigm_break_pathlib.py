from __future__ import annotations

import os
from pathlib import Path


def resolve_dataset_path(base: Path, name: str) -> Path:
    return base / ".argot" / f"{name}.jsonl"


def ensure_output_dir(output: Path) -> None:
    output.mkdir(parents=True, exist_ok=True)


def list_jsonl_files(directory: Path) -> list[Path]:
    return sorted(directory.glob("*.jsonl"))


def read_dataset(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def write_result(output_dir: Path, name: str, content: str) -> None:
    target = output_dir / f"{name}.json"
    target.write_text(content, encoding="utf-8")


def resolve_dataset_path_legacy(base: str, name: str) -> str:
    return os.path.join(base, ".argot", f"{name}.jsonl")


def ensure_output_dir_legacy(output: str) -> None:
    os.makedirs(output, exist_ok=True)


def list_jsonl_files_legacy(directory: str) -> list[str]:
    return sorted(os.path.join(directory, f) for f in os.listdir(directory) if f.endswith(".jsonl"))
