"""Collector for :class:`ImportEvidence` payloads.

Imports carry architectural identity: a ``mongoose`` line in a Postgres
repo is a much louder signal than an unfamiliar callee, so this collector
keeps the framing strictly factual — name the foreign specifiers and show
what the repo's typical top-level imports look like, without manufacturing
slot-aware substitution claims.
"""

from __future__ import annotations

from argot.scoring.evidence.types import EvidenceCorpus as _EvidenceCorpus
from argot.scoring.evidence.types import ImportEvidence, RarityStat

_COMMON_HERE_LIMIT = 10
_IMPORT_RARITY_NOUN = "module specifiers"
_IMPORT_RARITY_WHERE = "repo"


def collect_import_evidence(
    *,
    foreign_specifiers: list[str],
    evidence_corpus: _EvidenceCorpus,
) -> ImportEvidence:
    """Build the :class:`ImportEvidence` payload for an import-fired hit.

    ``foreign_specifiers`` is exactly the set the scorer flagged, in the
    order they appear in the hunk; the formatter caps and renders.
    """
    return ImportEvidence(
        foreign_specifiers=list(foreign_specifiers),
        rarity=RarityStat(
            flagged_count=0,
            attested_total=evidence_corpus.totals.import_specifiers_attested,
            noun=_IMPORT_RARITY_NOUN,
            where=_IMPORT_RARITY_WHERE,
        ),
        common_here=list(evidence_corpus.imports[:_COMMON_HERE_LIMIT]),
    )


__all__ = ["collect_import_evidence"]
