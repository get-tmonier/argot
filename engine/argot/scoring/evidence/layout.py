"""Shared layout primitives for the per-reason evidence formatters.

Centralised so all three formatters (BPE, import, call-receiver) render
overflow / frequency / floor / rarity wording identically. The PRD's UX
bundle (D6) is encoded as constants here — tweaking the user-visible
caps means changing this file, not chasing string literals across three
formatter modules.

Nothing in this module is per-reason; per-reason wording lives on
:class:`RarityStat` (``noun`` + ``where``) and on the formatter
implementations themselves.
"""

from __future__ import annotations

from collections.abc import Sequence

from argot.scoring.evidence.types import CommonEntry, RarityStat

# UX bundle (D6) — the user-visible caps and floors. Changes here propagate
# to every formatter; per-reason formatters never override.
TOP_K_NAMES = 3
TOP_K_COMMON_HERE = 3
COMMON_HERE_FLOOR = 3  # top-1 entry must occur ≥ this many times
RARITY_DENOMINATOR_FLOOR = 30  # show "0 of N" only when N ≥ this


def truncate_with_overflow(items: Sequence[str], k: int = TOP_K_NAMES) -> str:
    """Render ``items`` as ``"a, b, c"`` or ``"a, b, c (+N more)"``.

    ``k`` controls the visible head; the ``(+N more)`` suffix only appears
    when the input is strictly longer than ``k``.
    """
    if len(items) <= k:
        return ", ".join(items)
    head = ", ".join(items[:k])
    overflow = len(items) - k
    return f"{head} (+{overflow} more)"


def format_frequency(count: int) -> str:
    """Compact frequency suffix: ``847`` → ``"847×"``; ``3200`` → ``"3,200×"``.

    Comma-separated thousands keep large counts scannable; the ``×`` glyph
    distinguishes "this happened N times" from "the score is N" without
    needing the ``× `` to be parsed by anyone — it's pure typography.
    """
    return f"{count:,}×"


def should_show_common_here(entries: Sequence[CommonEntry]) -> bool:
    """Decide whether the ``common here:`` line is informative.

    Suppresses the line when even the top entry occurs fewer than
    :data:`COMMON_HERE_FLOOR` times. Below the floor, "common" is a
    misnomer and the line is more likely to mislead than orient.
    """
    return len(entries) > 0 and entries[0].count >= COMMON_HERE_FLOOR


def format_rarity(rarity: RarityStat) -> str:
    """Render the rarity fragment to the right of the ``↳`` names.

    Above :data:`RARITY_DENOMINATOR_FLOOR`: ``"0 of 12,400 identifiers in
    repo"``. Below it: ``"never seen in {where}"`` — anchoring on a tiny
    denominator overstates precision; "never seen" is honest about
    sample size.
    """
    if rarity.attested_total < RARITY_DENOMINATOR_FLOOR:
        return f"never seen in {rarity.where}"
    return f"{rarity.flagged_count} of {rarity.attested_total:,} {rarity.scope_label}"


def format_common_here_line(entries: Sequence[CommonEntry], k: int = TOP_K_COMMON_HERE) -> str:
    """Render the ``common here:`` line body — caller prepends the label.

    Each entry becomes ``"<name> (<count>×)"``; the ``+N more`` overflow
    suffix appears when ``len(entries) > k``. Returns the body string only;
    the ``"common here: "`` prefix is owned by the per-reason formatter so
    each can prepend an indent or ANSI dim wrapping consistently.
    """
    head = entries[:k]
    body = ", ".join(f"{e.name} ({format_frequency(e.count)})" for e in head)
    overflow = len(entries) - len(head)
    return f"{body} (+{overflow} more)" if overflow > 0 else body


__all__ = [
    "COMMON_HERE_FLOOR",
    "RARITY_DENOMINATOR_FLOOR",
    "TOP_K_COMMON_HERE",
    "TOP_K_NAMES",
    "format_common_here_line",
    "format_frequency",
    "format_rarity",
    "should_show_common_here",
    "truncate_with_overflow",
]
