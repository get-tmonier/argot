"""Per-reason ``Evidence`` payloads carried on :class:`ScoredHunk`.

The shape is uniform — names + a rarity stat + a small ``common here``
sample — but each reason scopes the data differently:

- BPE evidence: identifiers reconstructed from surprising token spans;
  rarity and common-here are repo-wide.
- Import evidence: the foreign top-level specifiers; rarity and common-
  here are repo-wide.
- Call-receiver evidence: the unattested callees; rarity and common-here
  are scoped to the hunk file's cluster, with a render-side label so the
  user sees ``in this cluster`` instead of ``in repo``.

Renderers must therefore dispatch on the runtime type, not on a string
``reason`` field — the data shape and label conventions are tied to the
type and would otherwise leak into ``check.py``.

The dataclasses are intentionally JSON-serialisable via
``dataclasses.asdict`` so the future ``--debug-evidence`` stderr dump and
a one-day ``--json`` output mode work without reshaping.
"""

from __future__ import annotations

from dataclasses import asdict, dataclass
from typing import Any


@dataclass(frozen=True)
class CommonEntry:
    """One entry on a ``common here:`` line — name + observed frequency.

    ``count`` is the raw observation count from the calibration corpus,
    used by the ``(847×)`` frequency suffix and by the floor logic that
    suppresses the line when even the top entry is too rare to inform.
    """

    name: str
    count: int


@dataclass(frozen=True)
class RarityStat:
    """Denominator-floored rarity context for the offending names.

    ``flagged_count`` is almost always 0 — by definition the flagged item
    is not attested in the slot it fired against. We surface it anyway so
    a hypothetical "1 of 12,400" case (e.g. a callee attested elsewhere
    but never in this cluster) renders honestly rather than silently.

    ``attested_total`` is the denominator the renderer prints when above
    the floor; below the floor it falls back to "never seen in {where}"
    so we don't anchor on a tiny denominator.

    ``noun`` and ``where`` are the two lexical halves of the rendered
    fragment — e.g. ``noun="identifiers"`` + ``where="repo"`` →
    "identifiers in repo"; ``noun="callees"`` + ``where="this cluster"``
    → "callees in this cluster". Splitting them lets the formatter build
    both the full ("0 of 12,400 identifiers in repo") and the floored
    ("never seen in this cluster") forms from the same data without
    string-parsing.
    """

    flagged_count: int
    attested_total: int
    noun: str
    where: str

    @property
    def scope_label(self) -> str:
        """Full natural-language fragment: ``"<noun> in <where>"``."""
        return f"{self.noun} in {self.where}"


@dataclass(frozen=True)
class BpeEvidence:
    """Evidence for a BPE-fired hit.

    ``surprising_identifiers`` is the BPE-piece-level top-K reconstructed
    back to whole identifiers via the surrounding source's character class
    (so ``mongo`` + ``ose`` → ``mongoose``). Rarity and common-here are
    repo-wide because BPE is a vocabulary-register signal, not a slot-
    aware one — claiming a slot here would manufacture fake precision.
    """

    surprising_identifiers: list[str]
    rarity: RarityStat
    common_here: list[CommonEntry]


@dataclass(frozen=True)
class ImportEvidence:
    """Evidence for an import-fired hit.

    ``foreign_specifiers`` is the set of top-level module names in the
    hunk that the import scorer marked foreign. Rarity and common-here
    are over the repo's distinct top-level import specifiers — an
    architectural-identity signal, not just vocabulary register.
    """

    foreign_specifiers: list[str]
    rarity: RarityStat
    common_here: list[CommonEntry]


@dataclass(frozen=True)
class CallReceiverEvidence:
    """Evidence for a call-receiver-fired hit.

    Cluster-scoped: rarity and common-here are computed against the hunk
    file's MinHash cluster's attested callees, not repo-wide. The
    ``scope_label`` on ``rarity`` reflects this so the rendered line
    reads ``callees in this cluster`` rather than the repo-wide framing
    used by the other two reasons.

    ``unfamiliar_callees`` is the deduped list of distinct callees in
    the hunk that were absent from the cluster's attested set.
    """

    unfamiliar_callees: list[str]
    rarity: RarityStat
    common_here: list[CommonEntry]


# Public alias — the runtime union of evidence payloads. Renderers and
# debug-evidence dump dispatch on the concrete type, never on a string.
Evidence = BpeEvidence | ImportEvidence | CallReceiverEvidence


@dataclass(frozen=True)
class EvidenceCorpusTotals:
    """Denominators for the rarity stats baked into :class:`EvidenceCorpus`.

    Identifier and import totals are repo-wide (singular ints); callee totals
    are scoped to each MinHash cluster the call-receiver scorer built, so
    cluster-conditional rarity statements (``0 of 1,247 callees in this
    cluster``) can pick the right denominator at score time.
    """

    import_specifiers_attested: int
    identifiers_attested: int
    callees_attested_by_cluster: dict[int, int]


@dataclass(frozen=True)
class EvidenceCorpus:
    """Pre-computed per-dimension top-N samples persisted in calibration JSON.

    Built once during ``argot calibrate`` from the same repo corpus the
    scorer sees, then loaded at check time by per-reason evidence collectors.
    Pre-computing here (rather than re-deriving at every check) keeps check
    fast and gives a single source of truth for what "common in this repo"
    means — both for ``common here:`` rendering and for the rarity denominator
    floors.

    ``callees_by_cluster`` keys are MinHash cluster ids matching the
    call-receiver scorer's ``cluster_attested`` keys; the call-receiver
    evidence collector picks the entry for the hunk-file's cluster.

    Designed to round-trip through ``to_json_dict`` → JSON →
    :meth:`from_json_dict` so calibration writers and check-time loaders
    stay symmetric.
    """

    imports: list[CommonEntry]
    identifiers: list[CommonEntry]
    callees_by_cluster: dict[int, list[CommonEntry]]
    totals: EvidenceCorpusTotals

    def to_json_dict(self) -> dict[str, Any]:
        """Return a dict suitable for ``json.dumps``.

        Uses ``dataclasses.asdict`` to flatten nested dataclasses into plain
        dicts; ``json.dumps`` will then coerce ``int`` cluster-id keys into
        strings, which :meth:`from_json_dict` reverses on the way back in.
        """
        return asdict(self)

    @classmethod
    def from_json_dict(cls, raw: dict[str, Any]) -> EvidenceCorpus:
        """Inverse of :meth:`to_json_dict`. JSON keys come back as strings;
        cluster ids are coerced back to ``int`` so they line up with the
        call-receiver scorer's cluster maps without surprising the caller.
        """
        totals_raw = raw["totals"]
        return cls(
            imports=[CommonEntry(**e) for e in raw["imports"]],
            identifiers=[CommonEntry(**e) for e in raw["identifiers"]],
            callees_by_cluster={
                int(k): [CommonEntry(**e) for e in v] for k, v in raw["callees_by_cluster"].items()
            },
            totals=EvidenceCorpusTotals(
                import_specifiers_attested=int(totals_raw["import_specifiers_attested"]),
                identifiers_attested=int(totals_raw["identifiers_attested"]),
                callees_attested_by_cluster={
                    int(k): int(v) for k, v in totals_raw["callees_attested_by_cluster"].items()
                },
            ),
        )


__all__ = [
    "BpeEvidence",
    "CallReceiverEvidence",
    "CommonEntry",
    "Evidence",
    "EvidenceCorpus",
    "EvidenceCorpusTotals",
    "ImportEvidence",
    "RarityStat",
]
