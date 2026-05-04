"""Reconstruct whole-identifier names from BPE-piece-level surprise spans.

UnixCoder's tokenizer is a fast Roberta tokenizer; ``return_offsets_mapping=True``
returns per-piece ``(start, end)`` character spans into the original source
string (Step 0 spike confirmed clean offsets). The BPE scorer flags pieces by
their per-token log-likelihood ratio against the repo corpus, but real
identifiers are usually split across multiple pieces (``mongoose`` →
``mongo`` + ``ose``; ``toStrictEqual`` → ``to`` + ``Strict`` + ``Equal``;
``authenticate_user`` → ``authenticate`` + ``_`` + ``user``).

Reconstruction expands each surprising piece's character span left and right
over the identifier character class in the source itself, then filters the
expanded substring to entries matching the identifier regex. This is more
robust than "merge adjacent top-K pieces" because it works even when only one
of an identifier's pieces lands in the top-K window.
"""

from __future__ import annotations

import re
from collections.abc import Callable
from typing import Any

# Identifier alphabet matches the typical programming-language rule: a letter
# or underscore start, then letters / digits / underscores. Adapters that
# need a different rule (e.g. JS allows ``$``) can wrap or shadow this.
_IDENTIFIER_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
_IDENT_CHARS: frozenset[str] = frozenset(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_"
)


def reconstruct_identifiers(source: str, spans: list[tuple[int, int]]) -> list[str]:
    """Expand each ``(start, end)`` char span to its enclosing identifier.

    For each span, walk left from ``start`` and right from ``end`` while
    characters belong to :data:`_IDENT_CHARS`. The resulting substring must
    match :data:`_IDENTIFIER_RE` — punctuation-only spans (``(``, ``);`` …)
    are dropped rather than rendered as gibberish. Output is deduped in
    occurrence order.
    """
    seen: set[str] = set()
    out: list[str] = []
    for start, end in spans:
        s = start
        while s > 0 and source[s - 1] in _IDENT_CHARS:
            s -= 1
        e = end
        while e < len(source) and source[e] in _IDENT_CHARS:
            e += 1
        candidate = source[s:e]
        if not candidate or not _IDENTIFIER_RE.match(candidate):
            continue
        if candidate in seen:
            continue
        seen.add(candidate)
        out.append(candidate)
    return out


def top_k_surprising_spans(
    hunk_source: str,
    tokenizer: Any,
    score_fn: Callable[[int], float],
    *,
    top_k: int,
    is_meaningful: Callable[[int], bool] | None = None,
) -> list[tuple[int, int]]:
    """Return the character spans of the top-``top_k`` surprising BPE pieces.

    Tokenizes ``hunk_source`` with offset-mapping, calls ``score_fn`` on each
    piece's token id, picks the ``top_k`` highest-scoring pieces, and returns
    their spans in original source order. Empty / whitespace-only spans
    (``(0, 0)`` markers some tokenizers emit) are skipped.

    ``is_meaningful`` filters out tokens we never want to surface — for
    example single-character punctuation pieces. The character-class
    expansion in :func:`reconstruct_identifiers` would already discard
    ``(`` etc., but skipping them up-front leaves more room in the top-K
    window for genuinely surprising identifier pieces.
    """
    if top_k <= 0 or not hunk_source:
        return []
    encoded = tokenizer(hunk_source, return_offsets_mapping=True, add_special_tokens=False)
    ids: list[int] = list(encoded["input_ids"])
    offsets: list[tuple[int, int]] = [tuple(o) for o in encoded["offset_mapping"]]
    if not ids:
        return []
    scored: list[tuple[int, int, float]] = [
        (s, e, score_fn(tok_id))
        for tok_id, (s, e) in zip(ids, offsets, strict=True)
        if s != e and (is_meaningful is None or is_meaningful(tok_id))
    ]
    if not scored:
        return []
    # Pick top-k by score descending; preserve source order in the output so
    # downstream rendering reflects the user's reading flow.
    threshold_idx = min(top_k, len(scored))
    by_score = sorted(scored, key=lambda t: -t[2])
    top_spans = {(s, e) for s, e, _ in by_score[:threshold_idx]}
    return [(s, e) for s, e, _ in scored if (s, e) in top_spans]


def surprising_identifiers(
    hunk_source: str,
    tokenizer: Any,
    score_fn: Callable[[int], float],
    *,
    top_k: int,
    max_identifiers: int,
    is_meaningful: Callable[[int], bool] | None = None,
) -> list[str]:
    """High-level helper: top-K surprising pieces → reconstructed identifiers.

    Reconstructs more than ``top_k`` whole identifiers in practice (one
    identifier can absorb multiple top-K pieces), then caps at
    ``max_identifiers`` to bound the rendered evidence line length.
    """
    spans = top_k_surprising_spans(
        hunk_source, tokenizer, score_fn, top_k=top_k, is_meaningful=is_meaningful
    )
    return reconstruct_identifiers(hunk_source, spans)[:max_identifiers]


__all__ = ["reconstruct_identifiers", "surprising_identifiers", "top_k_surprising_spans"]
