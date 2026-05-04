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
    back to whole identifiers (so ``mongo`` + ``ose`` → ``mongoose``), each
    paired with its repo-wide attestation count. The count makes the line
    self-explanatory: a reader scanning ``message (1,800×), opts (240×),
    proposed (5×)`` sees immediately which token is rare and which are
    familiar — the BPE flag is about the *sequence*, not necessarily about
    the words themselves.

    No rarity stat and no ``common here:`` line — both were render-time
    misdirection on real corpora. The identifier-attestation rarity stat
    ("0 of 71,811 identifiers in repo") was always 0 by definition (we
    only call ``surprising_identifiers`` what the scorer flagged) and read
    as if the words were foreign even when most of them appear thousands
    of times in the repo. The ``common here:`` line surfaced the global
    top-3 most-frequent identifiers (e.g. ``name, person, de`` on faker)
    which has nothing to do with the flagged tokens — repo-wide vocabulary
    statistics are not slot-comparable to a flagged hunk's content.
    """

    surprising_identifiers: list[CommonEntry]


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

    Import totals are repo-wide (singular int); callee totals are scoped to
    each MinHash cluster the call-receiver scorer built, so cluster-conditional
    rarity statements (``0 of 1,247 callees in this cluster``) can pick the
    right denominator at score time.

    No identifier total: BPE evidence shifted from a "0 of N identifiers"
    framing to per-token attestation counts (``proposed (5×)``), so the
    repo-wide identifier denominator is no longer rendered.
    """

    import_specifiers_attested: int
    callees_attested_by_cluster: dict[int, int]


@dataclass(frozen=True)
class EvidenceCorpus:
    """Pre-computed per-dimension samples persisted in calibration JSON.

    Built once during ``argot calibrate`` from the same repo corpus the
    scorer sees, then loaded at check time by per-reason evidence collectors.
    Pre-computing here (rather than re-deriving at every check) keeps check
    fast and gives a single source of truth for repo-wide vocabulary.

    ``identifiers`` is the **full** repo identifier count map (not top-N) so
    the BPE evidence collector can render any flagged identifier's
    attestation, including rare ones (``proposed (5×)``) that wouldn't fit
    in a top-N sample. The cost is JSON size — bounded by the number of
    distinct identifiers in the repo — but a flagged token whose count is
    missing from the map is exactly the case the user wants to see.

    ``imports`` and ``callees_by_cluster`` keep the top-N shape because the
    import / call-receiver evidence still uses ``common here:`` orientation
    lines; the slot-comparable framing reads sensibly there ("repo's stack:
    react, express, pg" / "this cluster's typical callees: ...").

    Designed to round-trip through ``to_json_dict`` → JSON →
    :meth:`from_json_dict` so calibration writers and check-time loaders
    stay symmetric.
    """

    imports: list[CommonEntry]
    identifiers: dict[str, int]
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
            identifiers={k: int(v) for k, v in raw["identifiers"].items()},
            callees_by_cluster={
                int(k): [CommonEntry(**e) for e in v] for k, v in raw["callees_by_cluster"].items()
            },
            totals=EvidenceCorpusTotals(
                import_specifiers_attested=int(totals_raw["import_specifiers_attested"]),
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
