from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Literal

import yaml

Language = Literal["python", "typescript"]


@dataclass(frozen=True)
class PR:
    pr: int
    sha: str


@dataclass(frozen=True)
class Target:
    name: str
    url: str
    language: Language
    prs: list[PR]


def load_targets(path: Path) -> list[Target]:
    raw = yaml.safe_load(path.read_text(encoding="utf-8"))
    out: list[Target] = []
    for t in raw["targets"]:
        prs = [PR(pr=int(p["pr"]), sha=str(p["sha"])) for p in t.get("prs", [])]
        out.append(
            Target(
                name=t["name"],
                url=t["url"],
                language=t["language"],
                prs=prs,
            )
        )
    return out
