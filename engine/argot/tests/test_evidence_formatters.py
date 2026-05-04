"""Snapshot-style tests for the per-reason evidence formatters.

Pin both the human-readable lines and the dispatch behaviour:

- The ``↳`` and ``common here:`` lines come out indented to align with
  the headline's score column / line column.
- Empty names suppress the ``↳`` line entirely (no empty rhetoric).
- The ``common here:`` floor and the rarity denominator floor compose
  correctly with the names line — every combination is exercised.
- The :func:`format_evidence` dispatcher routes by runtime type;
  routing the wrong type into a per-reason formatter is a loud failure,
  not silent garbage.
"""

from __future__ import annotations

import pytest

from argot.scoring.evidence.formatters import (
    BpeEvidenceFormatter,
    CallReceiverEvidenceFormatter,
    ImportEvidenceFormatter,
    format_evidence,
)
from argot.scoring.evidence.types import (
    BpeEvidence,
    CallReceiverEvidence,
    CommonEntry,
    ImportEvidence,
    RarityStat,
)


def _strip_ansi(s: str) -> str:
    """Remove ANSI escape sequences so the body text is comparable."""
    import re

    return re.sub(r"\x1b\[[0-9;]*m", "", s)


# --- BPE -------------------------------------------------------------------


class TestBpeEvidenceFormatter:
    def _evidence(
        self,
        names: list[str],
        common_here: list[CommonEntry] | None = None,
        rarity: RarityStat | None = None,
    ) -> BpeEvidence:
        return BpeEvidence(
            surprising_identifiers=names,
            rarity=rarity or RarityStat(0, 12_400, "identifiers", "repo"),
            common_here=common_here
            or [
                CommonEntry("useEffect", 320),
                CommonEntry("fetch", 180),
                CommonEntry("render", 120),
            ],
        )

    def test_full_render(self) -> None:
        ev = self._evidence(["toStrictEqual", "mockResolvedValue"])
        lines = [_strip_ansi(s) for s in BpeEvidenceFormatter().render(ev, use_color=False)]
        assert lines == [
            "     ↳ toStrictEqual, mockResolvedValue — 0 of 12,400 identifiers in repo",
            "       common here: useEffect (320×), fetch (180×), render (120×)",
        ]

    def test_overflow_in_names(self) -> None:
        ev = self._evidence(["a", "b", "c", "d", "e"])
        lines = [_strip_ansi(s) for s in BpeEvidenceFormatter().render(ev, use_color=False)]
        assert lines[0] == "     ↳ a, b, c (+2 more) — 0 of 12,400 identifiers in repo"

    def test_empty_names_suppresses_glyph_line(self) -> None:
        ev = self._evidence([])
        lines = [_strip_ansi(s) for s in BpeEvidenceFormatter().render(ev, use_color=False)]
        # Names line gone, common-here still printed.
        assert lines == ["       common here: useEffect (320×), fetch (180×), render (120×)"]

    def test_common_here_floor_suppresses(self) -> None:
        # top-1 count = 2 → below floor of 3 → suppress the line entirely
        ev = self._evidence(["x"], common_here=[CommonEntry("a", 2), CommonEntry("b", 1)])
        lines = [_strip_ansi(s) for s in BpeEvidenceFormatter().render(ev, use_color=False)]
        assert len(lines) == 1
        assert "common here:" not in lines[0]

    def test_rarity_denominator_floor(self) -> None:
        ev = self._evidence(
            ["frob"],
            rarity=RarityStat(0, 12, "identifiers", "repo"),  # below 30 → never seen
            common_here=[CommonEntry("a", 5), CommonEntry("b", 3)],
        )
        lines = [_strip_ansi(s) for s in BpeEvidenceFormatter().render(ev, use_color=False)]
        assert "never seen in repo" in lines[0]

    def test_color_wraps_each_line(self) -> None:
        ev = self._evidence(["x"])
        lines = BpeEvidenceFormatter().render(ev, use_color=True)
        # Both lines start with the dim escape and end with reset.
        assert all("\x1b[2m" in line and "\x1b[0m" in line for line in lines)

    def test_routing_wrong_type_is_loud(self) -> None:
        with pytest.raises(TypeError, match="BpeEvidenceFormatter"):
            BpeEvidenceFormatter().render(
                ImportEvidence(["pandas"], RarityStat(0, 47, "module specifiers", "repo"), []),
                use_color=False,
            )


# --- Imports --------------------------------------------------------------


class TestImportEvidenceFormatter:
    def test_full_render(self) -> None:
        ev = ImportEvidence(
            foreign_specifiers=["mongoose", "redis"],
            rarity=RarityStat(0, 47, "module specifiers", "repo"),
            common_here=[
                CommonEntry("react", 320),
                CommonEntry("express", 88),
                CommonEntry("pg", 47),
            ],
        )
        lines = [_strip_ansi(s) for s in ImportEvidenceFormatter().render(ev, use_color=False)]
        assert lines == [
            "     ↳ mongoose, redis — 0 of 47 module specifiers in repo",
            "       common here: react (320×), express (88×), pg (47×)",
        ]

    def test_routing_wrong_type_is_loud(self) -> None:
        with pytest.raises(TypeError, match="ImportEvidenceFormatter"):
            ImportEvidenceFormatter().render(
                BpeEvidence(["x"], RarityStat(0, 100, "identifiers", "repo"), []),
                use_color=False,
            )


# --- Call-receiver --------------------------------------------------------


class TestCallReceiverEvidenceFormatter:
    def test_full_render_cluster_scope(self) -> None:
        ev = CallReceiverEvidence(
            unfamiliar_callees=["legacy_decode_v2"],
            rarity=RarityStat(0, 1247, "callees", "this cluster"),
            common_here=[
                CommonEntry("logger.info", 3200),
                CommonEntry("db.query", 1800),
                CommonEntry("Result.ok", 900),
            ],
        )
        lines = [
            _strip_ansi(s) for s in CallReceiverEvidenceFormatter().render(ev, use_color=False)
        ]
        assert lines == [
            "     ↳ legacy_decode_v2 — 0 of 1,247 callees in this cluster",
            "       common here: logger.info (3,200×), db.query (1,800×), Result.ok (900×)",
        ]

    def test_repo_scope_fallback_when_no_cluster(self) -> None:
        # Singleton cluster path: collector has emitted where="repo", common=[]
        ev = CallReceiverEvidence(
            unfamiliar_callees=["foo"],
            rarity=RarityStat(0, 0, "callees", "repo"),
            common_here=[],
        )
        lines = [
            _strip_ansi(s) for s in CallReceiverEvidenceFormatter().render(ev, use_color=False)
        ]
        # 0 < 30 (denominator floor) → "never seen in repo".
        # No common_here entries → that line suppressed.
        assert lines == ["     ↳ foo — never seen in repo"]


# --- Dispatcher -----------------------------------------------------------


class TestFormatEvidenceDispatcher:
    def test_routes_bpe(self) -> None:
        ev = BpeEvidence(["x"], RarityStat(0, 100, "identifiers", "repo"), [])
        lines = format_evidence(ev, use_color=False)
        assert lines  # at least the names line
        assert "↳ x" in _strip_ansi(lines[0])

    def test_routes_import(self) -> None:
        ev = ImportEvidence(["pandas"], RarityStat(0, 47, "module specifiers", "repo"), [])
        lines = format_evidence(ev, use_color=False)
        assert "module specifiers in repo" in _strip_ansi(lines[0])

    def test_routes_call_receiver(self) -> None:
        ev = CallReceiverEvidence(["foo"], RarityStat(0, 1000, "callees", "this cluster"), [])
        lines = format_evidence(ev, use_color=False)
        assert "callees in this cluster" in _strip_ansi(lines[0])
