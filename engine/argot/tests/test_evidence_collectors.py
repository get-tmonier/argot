"""Unit tests for the per-reason evidence collectors.

The collectors translate scorer-side data shapes into the user-facing
:class:`Evidence` payloads. These tests verify the data-shape side
(rarity uses the right scope, ``common_here`` is sliced from the right
dimension, cluster fallback works) without touching the formatter layer
— that's covered separately in ``test_evidence_formatters.py``.
"""

from __future__ import annotations

from argot.scoring.evidence.call_receiver import collect_call_receiver_evidence
from argot.scoring.evidence.imports import collect_import_evidence
from argot.scoring.evidence.types import (
    CommonEntry,
    EvidenceCorpus,
    EvidenceCorpusTotals,
)


def _corpus(
    *,
    imports: list[CommonEntry] | None = None,
    identifiers: list[CommonEntry] | None = None,
    callees_by_cluster: dict[int, list[CommonEntry]] | None = None,
    import_total: int = 47,
    identifier_total: int = 12_400,
    callee_totals: dict[int, int] | None = None,
) -> EvidenceCorpus:
    return EvidenceCorpus(
        imports=imports or [CommonEntry("react", 320), CommonEntry("express", 88)],
        identifiers=identifiers or [CommonEntry("useEffect", 320)],
        callees_by_cluster=callees_by_cluster
        or {0: [CommonEntry("logger.info", 3200), CommonEntry("db.query", 1800)]},
        totals=EvidenceCorpusTotals(
            import_specifiers_attested=import_total,
            identifiers_attested=identifier_total,
            callees_attested_by_cluster=callee_totals or {0: 1247},
        ),
    )


# --- Imports ---------------------------------------------------------------


class TestCollectImportEvidence:
    def test_packs_foreign_specifiers_and_repo_imports(self) -> None:
        ev = collect_import_evidence(
            foreign_specifiers=["mongoose", "redis"],
            evidence_corpus=_corpus(),
        )
        assert ev.foreign_specifiers == ["mongoose", "redis"]
        assert ev.rarity.flagged_count == 0
        assert ev.rarity.attested_total == 47
        assert ev.rarity.scope_label == "module specifiers in repo"
        assert ev.common_here == [
            CommonEntry("react", 320),
            CommonEntry("express", 88),
        ]

    def test_caps_common_here_at_collector_limit(self) -> None:
        big = [CommonEntry(f"m{i}", 100 - i) for i in range(20)]
        ev = collect_import_evidence(
            foreign_specifiers=["x"],
            evidence_corpus=_corpus(imports=big),
        )
        # Collector caps at 10 (formatter further caps to 3 + (+N more))
        assert len(ev.common_here) == 10


# --- Call-receiver --------------------------------------------------------


class TestCollectCallReceiverEvidence:
    def test_uses_cluster_scope_when_cluster_known(self) -> None:
        ev = collect_call_receiver_evidence(
            unattested_callees=["legacy_decode_v2"],
            cluster_id=0,
            evidence_corpus=_corpus(),
        )
        assert ev.rarity.scope_label == "callees in this cluster"
        assert ev.rarity.attested_total == 1247
        assert ev.common_here[0].name == "logger.info"

    def test_falls_back_to_repo_scope_when_cluster_unknown(self) -> None:
        ev = collect_call_receiver_evidence(
            unattested_callees=["frob"],
            cluster_id=None,
            evidence_corpus=_corpus(),
        )
        assert ev.rarity.scope_label == "callees in repo"
        assert ev.rarity.attested_total == 0
        assert ev.common_here == []

    def test_falls_back_when_cluster_id_not_in_map(self) -> None:
        # Cluster id 7 not present → fallback to repo scope.
        ev = collect_call_receiver_evidence(
            unattested_callees=["frob"],
            cluster_id=7,
            evidence_corpus=_corpus(),
        )
        assert ev.rarity.where == "repo"
        assert ev.common_here == []

    def test_caps_common_here(self) -> None:
        big_cluster = {0: [CommonEntry(f"c{i}", 100 - i) for i in range(20)]}
        ev = collect_call_receiver_evidence(
            unattested_callees=["x"],
            cluster_id=0,
            evidence_corpus=_corpus(callees_by_cluster=big_cluster),
        )
        assert len(ev.common_here) == 10

    def test_unfamiliar_callees_copied_not_aliased(self) -> None:
        original = ["a", "b"]
        ev = collect_call_receiver_evidence(
            unattested_callees=original,
            cluster_id=0,
            evidence_corpus=_corpus(),
        )
        original.append("c")
        # Collector must defensively copy — Evidence is frozen, but the
        # nested list isn't deep-frozen, so we want isolation by convention.
        assert ev.unfamiliar_callees == ["a", "b"]
