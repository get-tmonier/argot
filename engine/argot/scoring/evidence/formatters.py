"""Per-reason renderers that turn an :class:`Evidence` payload into lines.

Each reason gets its own formatter class — the polymorphism is deliberate
(see PRD §D4): the data shapes already differ today, and per-reason
divergence is expected once Tier 3 lands. Keeping the formatters
separate means Tier 3 only touches the affected file.

All shared layout decisions (truncation, frequency formatting, floor
thresholds, indentation) live in :mod:`evidence.layout`. The formatters
own only the per-reason "where to read names from" choice and the
no-color / color rendering toggle.
"""

from __future__ import annotations

from typing import Protocol

from argot.scoring.evidence.layout import (
    TOP_K_NAMES,
    format_common_here_line,
    format_frequency,
    format_rarity,
    should_show_common_here,
    truncate_with_overflow,
)
from argot.scoring.evidence.types import (
    BpeEvidence,
    CallReceiverEvidence,
    CommonEntry,
    Evidence,
    ImportEvidence,
    RarityStat,
)

# Two ANSI codes — kept here rather than imported from ``check.py`` to avoid
# a render → check dependency cycle. Evidence rendering is dim by design:
# it's secondary information sitting under each headline hit.
_DIM = "\x1b[2m"
_RESET = "\x1b[0m"

# Indents picked to align the ``↳`` glyph under the score column of the
# headline (5 spaces) and the ``common here:`` body two characters deeper
# (7 spaces) so the eye reads a clear two-column secondary strip.
_NAMES_INDENT = "     "
_COMMON_INDENT = "       "

_GLYPH = "↳"
_COMMON_PREFIX = "common here:"


def _dim(text: str, *, use_color: bool) -> str:
    """Wrap ``text`` in dim ANSI when colour is enabled, else return as-is."""
    return f"{_DIM}{text}{_RESET}" if use_color else text


def _names_line(names: list[str], rarity: RarityStat, *, use_color: bool) -> str | None:
    """Render the ``↳`` line body or return ``None`` to suppress.

    No names → no line; the headline + the optional ``common here:`` are
    still informative on their own, and printing ``↳ never seen in repo``
    with no names would be empty rhetoric.
    """
    if not names:
        return None
    body = f"{truncate_with_overflow(names)} — {format_rarity(rarity)}"
    return _NAMES_INDENT + _dim(f"{_GLYPH} {body}", use_color=use_color)


def _common_here_line(entries: list[CommonEntry], *, use_color: bool) -> str:
    """Render the ``common here: ...`` line body. Caller pre-checks the floor."""
    body = format_common_here_line(entries)
    return _COMMON_INDENT + _dim(f"{_COMMON_PREFIX} {body}", use_color=use_color)


class EvidenceFormatter(Protocol):
    """Structural interface every per-reason formatter satisfies."""

    def render(self, evidence: Evidence, *, use_color: bool) -> list[str]: ...


class BpeEvidenceFormatter:
    """Render :class:`BpeEvidence` to a single ``↳`` line of per-token counts.

    Unlike import / call-receiver evidence, BPE has no slot-comparable
    secondary line: the global ``common here:`` orientation
    (``name (3,014×), person (1,210×), de (1,163×)``) wasn't slot-comparable
    to a flagged hunk's content, and the rarity stat (``0 of 71,811
    identifiers in repo``) read as a foreign-vocabulary claim even when the
    flagged tokens were ubiquitous in the codebase. Both lines are dropped
    in favour of inline counts on the names themselves —
    ``message (1,800×), opts (240×), proposed (5×)`` is honest, slot-tied,
    and immediately actionable.
    """

    def render(self, evidence: Evidence, *, use_color: bool) -> list[str]:
        # Narrow at the boundary so the protocol's wider type doesn't leak
        # into the body — a misroute by the dispatcher fails loudly here.
        if not isinstance(evidence, BpeEvidence):
            raise TypeError(f"BpeEvidenceFormatter received {type(evidence).__name__}")
        if not evidence.surprising_identifiers:
            return []
        head = evidence.surprising_identifiers[:TOP_K_NAMES]
        body = ", ".join(f"{e.name} ({format_frequency(e.count)})" for e in head)
        overflow = len(evidence.surprising_identifiers) - len(head)
        if overflow > 0:
            body = f"{body} (+{overflow} more)"
        return [_NAMES_INDENT + _dim(f"{_GLYPH} {body}", use_color=use_color)]


class ImportEvidenceFormatter:
    """Render :class:`ImportEvidence` to ≤ 2 lines under an import-fired hit."""

    def render(self, evidence: Evidence, *, use_color: bool) -> list[str]:
        if not isinstance(evidence, ImportEvidence):
            raise TypeError(f"ImportEvidenceFormatter received {type(evidence).__name__}")
        out: list[str] = []
        names_line = _names_line(evidence.foreign_specifiers, evidence.rarity, use_color=use_color)
        if names_line is not None:
            out.append(names_line)
        if should_show_common_here(evidence.common_here):
            out.append(_common_here_line(evidence.common_here, use_color=use_color))
        return out


class CallReceiverEvidenceFormatter:
    """Render :class:`CallReceiverEvidence` to ≤ 2 lines under a CR-fired hit."""

    def render(self, evidence: Evidence, *, use_color: bool) -> list[str]:
        if not isinstance(evidence, CallReceiverEvidence):
            raise TypeError(f"CallReceiverEvidenceFormatter received {type(evidence).__name__}")
        out: list[str] = []
        names_line = _names_line(evidence.unfamiliar_callees, evidence.rarity, use_color=use_color)
        if names_line is not None:
            out.append(names_line)
        if should_show_common_here(evidence.common_here):
            out.append(_common_here_line(evidence.common_here, use_color=use_color))
        return out


def format_evidence(evidence: Evidence, *, use_color: bool) -> list[str]:
    """Single-entrypoint dispatcher for the renderer.

    Routes the runtime type to the matching formatter — keeps the
    rendering side of ``check.py`` blind to per-reason rendering details.
    Returns ``[]`` for unrecognised payload types so a future evidence
    type slipped in without a formatter degrades to "no evidence shown"
    rather than crashing the renderer.
    """
    if isinstance(evidence, BpeEvidence):
        return BpeEvidenceFormatter().render(evidence, use_color=use_color)
    if isinstance(evidence, ImportEvidence):
        return ImportEvidenceFormatter().render(evidence, use_color=use_color)
    if isinstance(evidence, CallReceiverEvidence):
        return CallReceiverEvidenceFormatter().render(evidence, use_color=use_color)
    return []


__all__ = [
    "BpeEvidenceFormatter",
    "CallReceiverEvidenceFormatter",
    "EvidenceFormatter",
    "ImportEvidenceFormatter",
    "format_evidence",
]
