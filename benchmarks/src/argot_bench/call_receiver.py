"""Call-receiver scorer (era 6, research phase).

Presence-based Stage 1.5 predicate: flags hunks that introduce
call-expression receivers (full dotted callee strings) absent from the
repo's own call sites. Lives inside the benchmark sandbox; production
scorer in ``engine/argot/scoring/`` is untouched on this branch.

See docs/superpowers/specs/2026-04-24-era6-call-receiver.md for design.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Literal

Language = Literal["python", "typescript"]


def extract_callees(source: str, language: Language) -> list[str | None]:
    """Return dotted-callee signatures for every call-expression in *source*.

    Each call-expression maps to either a dotted string (``"Math.random"``,
    ``"app.route"``, ``"fetch"``) or ``None`` when the callee bottoms out
    at a non-identifier (another call, subscript, parenthesized expression).
    ``None`` entries are counted for auditing but excluded from set membership.

    Returns ``[]`` on parse error or empty source.
    """
    raise NotImplementedError


@dataclass(frozen=True)
class CallReceiverResult:
    """Result of scoring a hunk's call-expression receivers against the attested set."""

    unattested: tuple[str, ...]
    flagged: bool


class CallReceiverScorer:
    """Stage-1.5 presence-based scorer.

    Fit: scan *model_a_files*, union all non-None callees into a frozenset.
    Score: extract callees from a hunk, flag if ``len(unattested) >= k``.
    """

    def __init__(
        self,
        model_a_files: list[Path],
        *,
        language: Language,
        k: int = 1,
    ) -> None:
        raise NotImplementedError

    def score_hunk(self, hunk_content: str) -> CallReceiverResult:
        raise NotImplementedError
