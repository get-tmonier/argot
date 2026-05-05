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
    SourceSpan,
)


def _strip_ansi(s: str) -> str:
    """Remove ANSI escape sequences so the body text is comparable."""
    import re

    return re.sub(r"\x1b\[[0-9;]*m", "", s)


# --- BPE -------------------------------------------------------------------


class TestBpeEvidenceFormatter:
    """BPE evidence is a single ``↳`` line of per-token attestation counts.

    No ``common here:`` orientation and no ``0 of N`` rarity denominator —
    both were render-time misdirection on real corpora. The inline counts
    let the user see at a glance which flagged token is genuinely rare.
    """

    @staticmethod
    def _evidence(entries: list[CommonEntry]) -> BpeEvidence:
        return BpeEvidence(surprising_identifiers=entries)

    def test_full_render(self) -> None:
        ev = self._evidence(
            [CommonEntry("message", 1800), CommonEntry("opts", 240), CommonEntry("proposed", 5)]
        )
        lines = [_strip_ansi(s) for s in BpeEvidenceFormatter().render(ev, use_color=False)]
        assert lines == [
            "     ↳ message (1,800×), opts (240×), proposed (5×)",
        ]

    def test_overflow(self) -> None:
        ev = self._evidence([CommonEntry(c, 1) for c in "abcde"])
        lines = [_strip_ansi(s) for s in BpeEvidenceFormatter().render(ev, use_color=False)]
        assert lines == ["     ↳ a (1×), b (1×), c (1×) (+2 more)"]

    def test_zero_count_renders_honestly(self) -> None:
        """A genuinely novel identifier (not in repo at all) still appears.

        The collector emits ``count=0`` for tokens absent from
        ``EvidenceCorpus.identifiers`` — the zero is the signal, not noise.
        """
        ev = self._evidence([CommonEntry("frobnicate", 0)])
        lines = [_strip_ansi(s) for s in BpeEvidenceFormatter().render(ev, use_color=False)]
        assert lines == ["     ↳ frobnicate (0×)"]

    def test_empty_names_suppresses_line(self) -> None:
        ev = self._evidence([])
        lines = BpeEvidenceFormatter().render(self._evidence([]), use_color=False)
        assert lines == []
        del ev  # silence unused-var

    def test_color_wraps_line(self) -> None:
        ev = self._evidence([CommonEntry("x", 5)])
        lines = BpeEvidenceFormatter().render(ev, use_color=True)
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
                BpeEvidence([CommonEntry("x", 1)]),
                use_color=False,
            )

    def test_line_annotation_uses_file_line(self) -> None:
        """``foreign_specifier_spans`` carries hunk-relative spans; the
        formatter shifts ``span.line`` by ``hunk_start_line`` so the
        rendered annotation reads as a file line — the number the user
        sees in their editor.
        """
        ev = ImportEvidence(
            foreign_specifiers=["msgspec"],
            rarity=RarityStat(0, 120, "module specifiers", "repo"),
            common_here=[CommonEntry("typing", 292)],
            foreign_specifier_spans={"msgspec": SourceSpan(line=7, col_start=7, col_end=14)},
        )
        # Hunk starts at file line 1 → annotation = L7 (the file line)
        lines = [
            _strip_ansi(s)
            for s in ImportEvidenceFormatter().render(ev, use_color=False, hunk_start_line=1)
        ]
        assert lines[0] == "     ↳ msgspec (L7) — 0 of 120 module specifiers in repo"
        # Hunk starts at file line 100 → annotation should be L106 (100 + 7 - 1)
        lines = [
            _strip_ansi(s)
            for s in ImportEvidenceFormatter().render(ev, use_color=False, hunk_start_line=100)
        ]
        assert lines[0] == "     ↳ msgspec (L106) — 0 of 120 module specifiers in repo"

    def test_line_annotation_omitted_when_unknown(self) -> None:
        """If the scorer couldn't capture a span for a specifier (parse
        error, edge case), the formatter falls back to the bare name.
        """
        ev = ImportEvidence(
            foreign_specifiers=["mongoose"],
            rarity=RarityStat(0, 47, "module specifiers", "repo"),
            common_here=[],
            foreign_specifier_spans={},  # no span info
        )
        lines = [
            _strip_ansi(s)
            for s in ImportEvidenceFormatter().render(ev, use_color=False, hunk_start_line=1)
        ]
        assert "mongoose" in lines[0]
        assert "(L" not in lines[0]


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
        ev = BpeEvidence([CommonEntry("x", 5)])
        lines = format_evidence(ev, use_color=False)
        assert lines
        assert "↳ x (5×)" in _strip_ansi(lines[0])

    def test_routes_import(self) -> None:
        ev = ImportEvidence(["pandas"], RarityStat(0, 47, "module specifiers", "repo"), [])
        lines = format_evidence(ev, use_color=False)
        assert "module specifiers in repo" in _strip_ansi(lines[0])

    def test_routes_call_receiver(self) -> None:
        ev = CallReceiverEvidence(["foo"], RarityStat(0, 1000, "callees", "this cluster"), [])
        lines = format_evidence(ev, use_color=False)
        assert "callees in this cluster" in _strip_ansi(lines[0])
