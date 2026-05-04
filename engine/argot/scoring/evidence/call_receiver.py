"""Collector for :class:`CallReceiverEvidence` payloads.

Cluster-scoped: the call-receiver scorer groups files by callee-bag
similarity, and a callee that's typical in one cluster (a logger in a
service module) can be unfamiliar in another (a CLI entry point). The
``common here:`` slice and the rarity denominator therefore both come
from the hunk file's MinHash cluster, not the repo as a whole.

Singleton / unknown clusters fall back to a repo-empty framing rather
than printing a wrong cluster's data — see the docstring on the
collector for the exact contract.
"""

from __future__ import annotations

from argot.scoring.evidence.types import CallReceiverEvidence, RarityStat
from argot.scoring.evidence.types import EvidenceCorpus as _EvidenceCorpus

_COMMON_HERE_LIMIT = 10
_CALLEE_NOUN = "callees"
_CLUSTER_WHERE = "this cluster"
_REPO_WHERE = "repo"


def collect_call_receiver_evidence(
    *,
    unattested_callees: list[str],
    cluster_id: int | None,
    evidence_corpus: _EvidenceCorpus,
) -> CallReceiverEvidence:
    """Build :class:`CallReceiverEvidence` scoped to the hunk's cluster.

    ``cluster_id`` is the cluster the call-receiver scorer assigned to the
    hunk's file (``None`` for files that fell through the static lookup
    *and* the Jaccard fallback — singleton clusters or files with empty
    callee bags). When ``None``, ``common_here`` is empty and the rarity
    denominator is 0; the formatter then prints "never seen in repo"
    instead of "0 of N in this cluster" — honest about the missing
    cluster context rather than printing a misleading denominator.
    """
    if cluster_id is not None and cluster_id in evidence_corpus.callees_by_cluster:
        common = list(evidence_corpus.callees_by_cluster[cluster_id][:_COMMON_HERE_LIMIT])
        denom = evidence_corpus.totals.callees_attested_by_cluster.get(cluster_id, 0)
        where = _CLUSTER_WHERE
    else:
        common = []
        denom = 0
        where = _REPO_WHERE
    return CallReceiverEvidence(
        unfamiliar_callees=list(unattested_callees),
        rarity=RarityStat(
            flagged_count=0,
            attested_total=denom,
            noun=_CALLEE_NOUN,
            where=where,
        ),
        common_here=common,
    )


__all__ = ["collect_call_receiver_evidence"]
