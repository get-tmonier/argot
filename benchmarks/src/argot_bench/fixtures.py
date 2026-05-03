from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any, Literal

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
    # Era-14 Fix A: optional host-file injection for ML feature extraction.
    # ``host_file``: path relative to the corpus repo root (e.g.
    #   ``"src/modules/person/first-name.ts"``).
    # ``host_inject_at_line``: 1-indexed line in the host file BEFORE which the
    #   catalog file's full content is spliced.
    # Both fields must be present together OR both absent. When set, the ML
    # feature extractor scores the synthesized (catalog-into-host) content
    # instead of the standalone catalog file, eliminating the catalog
    # fixture-shape leak documented in era14-status.md.
    host_file: str | None = None
    host_inject_at_line: int | None = None


@dataclass(frozen=True)
class Catalog:
    corpus: str
    language: Language
    categories: list[str]
    injection_hosts: list[PRHost]
    fixtures: list[Fixture]


def _parse_fixture(raw: dict[str, Any]) -> Fixture:
    """Parse a single fixture dict, validating host_file/host_inject_at_line pairing."""
    host_file = raw.get("host_file")
    host_inject_at_line_raw = raw.get("host_inject_at_line")
    # Era-14 Fix A: enforce both-or-neither for host injection fields.
    if (host_file is None) != (host_inject_at_line_raw is None):
        raise ValueError(
            f"fixture {raw.get('id')!r}: host_file and host_inject_at_line must be "
            "specified together (got "
            f"host_file={host_file!r}, host_inject_at_line={host_inject_at_line_raw!r})."
        )
    host_inject_at_line: int | None = (
        int(host_inject_at_line_raw) if host_inject_at_line_raw is not None else None
    )
    if host_inject_at_line is not None and host_inject_at_line < 1:
        raise ValueError(
            f"fixture {raw.get('id')!r}: host_inject_at_line must be >= 1 "
            f"(got {host_inject_at_line})."
        )
    return Fixture(
        id=raw["id"],
        file=raw["file"],
        category=raw["category"],
        hunk_start_line=int(raw["hunk_start_line"]),
        hunk_end_line=int(raw["hunk_end_line"]),
        rationale=raw.get("rationale", ""),
        difficulty=raw.get("difficulty"),
        host_file=str(host_file) if host_file is not None else None,
        host_inject_at_line=host_inject_at_line,
    )


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
        fixtures=[_parse_fixture(f) for f in raw.get("fixtures", [])],
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
