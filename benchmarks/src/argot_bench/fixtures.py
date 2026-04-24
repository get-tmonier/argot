from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Literal

import yaml

Language = Literal["python", "typescript"]


@dataclass(frozen=True)
class PRHost:
    pr: int
    sha: str


@dataclass(frozen=True)
class Fixture:
    id: str
    file: str                # path relative to catalog dir
    category: str
    hunk_start_line: int
    hunk_end_line: int
    rationale: str = ""
    difficulty: Literal["easy", "medium", "hard", "uncaught"] | None = None


@dataclass(frozen=True)
class Catalog:
    corpus: str
    language: Language
    categories: list[str]
    injection_hosts: list[PRHost]
    fixtures: list[Fixture]


def load_catalog(dir_path: Path) -> Catalog:
    raw = yaml.safe_load((dir_path / "manifest.yaml").read_text(encoding="utf-8"))
    return Catalog(
        corpus=raw["corpus"],
        language=raw["language"],
        categories=list(raw["categories"]),
        injection_hosts=[
            PRHost(pr=int(h["pr"]), sha=str(h["sha"]))
            for h in raw.get("injection_hosts", [])
        ],
        fixtures=[
            Fixture(
                id=f["id"],
                file=f["file"],
                category=f["category"],
                hunk_start_line=int(f["hunk_start_line"]),
                hunk_end_line=int(f["hunk_end_line"]),
                rationale=f.get("rationale", ""),
                difficulty=f.get("difficulty"),
            )
            for f in raw.get("fixtures", [])
        ],
    )


def scan_all_catalogs(catalogs_root: Path) -> list[Catalog]:
    return [
        load_catalog(d)
        for d in sorted(catalogs_root.iterdir())
        if d.is_dir() and (d / "manifest.yaml").exists()
    ]


def read_hunk(catalog_dir: Path, fixture: Fixture) -> tuple[str, str]:
    """Return (full_file_source, hunk_content) for a fixture."""
    full = (catalog_dir / fixture.file).read_text(encoding="utf-8")
    lines = full.splitlines()
    hunk = "\n".join(lines[fixture.hunk_start_line - 1 : fixture.hunk_end_line])
    return full, hunk
