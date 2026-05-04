"""Collector for :class:`BpeEvidence` payloads.

The BPE scorer's verdict is a single max-over-tokens log-likelihood ratio —
useless for explaining *which* tokens drove it. This collector reconstructs
the offending identifiers from the surprising token spans and pairs each
with its repo-wide attestation count so the rendered line is self-explanatory:
``message (1,800×), opts (240×), proposed (5×)`` immediately tells the reader
which token is rare and which are familiar.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

from argot.scoring.evidence.bpe_reconstruction import surprising_identifiers
from argot.scoring.evidence.types import BpeEvidence, CommonEntry
from argot.scoring.evidence.types import EvidenceCorpus as _EvidenceCorpus

# Generous cap at the collector layer; the formatter applies the user-
# visible ``top-3 + (+N more)`` truncation. Sized so the formatter has
# enough headroom to compute "+N more" overflow correctly without forcing
# the collector to know the rendering convention.
_MAX_SURPRISING_PIECES = 8
_MAX_IDENTIFIERS = 8


def collect_bpe_evidence(
    *,
    hunk_source: str,
    tokenizer: Any,
    score_fn: Callable[[int], float],
    is_meaningful: Callable[[int], bool] | None,
    evidence_corpus: _EvidenceCorpus,
) -> BpeEvidence:
    """Build the :class:`BpeEvidence` payload for a BPE-fired hit.

    ``hunk_source`` is the prose-blanked hunk text that was actually scored
    — passing the raw hunk would let comments / docstrings into the
    surprising-identifier list, contradicting how the scorer treats them.

    ``score_fn`` is the per-token surprise function the scorer uses
    internally; ``is_meaningful`` is the same single-char filter the scorer
    applies before the max — both are passed in so this collector stays
    decoupled from the scorer class and is straightforward to unit-test
    with synthetic inputs.

    Each surprising identifier is paired with its repo-wide attestation
    count from ``evidence_corpus.identifiers``. Tokens absent from the map
    render as ``count=0``: a genuinely novel identifier should read as
    ``proposed (0×)`` rather than be silently dropped — the zero is the
    signal.
    """
    names = surprising_identifiers(
        hunk_source,
        tokenizer,
        score_fn,
        top_k=_MAX_SURPRISING_PIECES,
        max_identifiers=_MAX_IDENTIFIERS,
        is_meaningful=is_meaningful,
    )
    counts = evidence_corpus.identifiers
    return BpeEvidence(
        surprising_identifiers=[CommonEntry(name=n, count=counts.get(n, 0)) for n in names],
    )


__all__ = ["collect_bpe_evidence"]
