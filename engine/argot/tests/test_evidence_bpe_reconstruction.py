"""Unit tests for BPE-piece-level identifier reconstruction.

UnixCoder splits ``mongoose`` into ``mongo`` + ``ose`` (and similar);
:func:`reconstruct_identifiers` recovers the whole identifier by walking
the source's character class outward from each surprising piece's span.

These tests cover:
- The basic "two pieces of one identifier" case.
- Punctuation pieces that don't reconstruct to anything (filtered out).
- Dedup in occurrence order (one identifier referenced from multiple
  surprising pieces still appears once).
- Top-K selection in :func:`top_k_surprising_spans`.
- The ``is_meaningful`` filter survives propagation.
"""

from __future__ import annotations

from collections.abc import Callable

import pytest

from argot.scoring.evidence.bpe_reconstruction import (
    reconstruct_identifiers,
    surprising_identifiers,
    top_k_surprising_spans,
)


class _FakeBatchEncoding(dict[str, list[object]]):
    """Tokenizer-output shim: subclasses dict so ``out["input_ids"]`` works."""


class _FakeTokenizer:
    """Tokenizer stub: returns pre-baked (id, span) pairs.

    Real tokenizers return ``BatchEncoding``; the production code only reads
    ``["input_ids"]`` and ``["offset_mapping"]`` so a dict is sufficient.
    """

    def __init__(self, pairs: list[tuple[int, tuple[int, int]]]) -> None:
        self._pairs = pairs

    def __call__(
        self,
        source: str,  # noqa: ARG002 — match the production tokenizer's signature
        *,
        return_offsets_mapping: bool,
        add_special_tokens: bool,
    ) -> _FakeBatchEncoding:
        assert return_offsets_mapping is True
        assert add_special_tokens is False
        return _FakeBatchEncoding(
            input_ids=[i for i, _ in self._pairs],
            offset_mapping=[s for _, s in self._pairs],
        )


# ---------------------------------------------------------------------------
# reconstruct_identifiers
# ---------------------------------------------------------------------------


class TestReconstructIdentifiers:
    def test_expands_single_piece_to_full_identifier(self) -> None:
        # source: "    return mongoose.connect()"
        #          0123456789012345678901234567890
        source = "    return mongoose.connect()"
        # Span "mongo" → expand to "mongoose"
        spans = [(11, 16)]  # 'mongo'
        assert reconstruct_identifiers(source, spans) == ["mongoose"]

    def test_two_pieces_collapse_into_one_identifier(self) -> None:
        source = "expect(result).toStrictEqual(other);"
        # 'to' span at 15..17, 'Strict' span at 17..23, 'Equal' at 23..28
        spans = [(15, 17), (17, 23), (23, 28)]
        # All three expand to 'toStrictEqual'; dedup → one entry.
        assert reconstruct_identifiers(source, spans) == ["toStrictEqual"]

    def test_punctuation_only_span_dropped(self) -> None:
        source = "x = (foo) + bar;"
        # Span on '(' (4..5) → expansion stops because '(' isn't an ident char.
        spans = [(4, 5), (12, 15)]
        assert reconstruct_identifiers(source, spans) == ["bar"]

    def test_dedup_preserves_first_occurrence_order(self) -> None:
        source = "alpha bravo alpha"
        # Two surprising pieces both expand to 'alpha'; order in source
        # determines which appears first; dedup keeps just one entry.
        spans = [(0, 3), (12, 17)]
        assert reconstruct_identifiers(source, spans) == ["alpha"]

    def test_underscore_part_of_identifier(self) -> None:
        source = "def authenticate_user(req): pass"
        # Span on '_' alone → expand both ways → 'authenticate_user'
        spans = [(16, 17)]
        assert reconstruct_identifiers(source, spans) == ["authenticate_user"]

    def test_digits_inside_identifier(self) -> None:
        source = "value = decode_v2(payload)"
        spans = [(8, 14)]  # 'decode'
        assert reconstruct_identifiers(source, spans) == ["decode_v2"]

    def test_identifier_starting_with_digit_rejected(self) -> None:
        # An expansion that produces something starting with a digit fails the
        # identifier regex → dropped, not rendered as gibberish.
        source = "value = 42"
        spans = [(8, 10)]  # '42'
        assert reconstruct_identifiers(source, spans) == []

    def test_empty_source_returns_empty(self) -> None:
        assert reconstruct_identifiers("", []) == []
        assert reconstruct_identifiers("", [(0, 5)]) == []


# ---------------------------------------------------------------------------
# top_k_surprising_spans
# ---------------------------------------------------------------------------


class TestTopKSurprisingSpans:
    def test_picks_top_k_by_score_returns_in_source_order(self) -> None:
        source = "a b c d e"
        # ids: arbitrary; spans go left-to-right with single-char content
        pairs = [
            (1, (0, 1)),  # 'a' score 1
            (2, (2, 3)),  # 'b' score 5 ← top
            (3, (4, 5)),  # 'c' score 2
            (4, (6, 7)),  # 'd' score 9 ← top
            (5, (8, 9)),  # 'e' score 0
        ]
        scores = {1: 1.0, 2: 5.0, 3: 2.0, 4: 9.0, 5: 0.0}
        tok = _FakeTokenizer(pairs)
        spans = top_k_surprising_spans(source, tok, lambda i: scores[i], top_k=2)
        # 'b' span (2,3) and 'd' span (6,7) — order preserved.
        assert spans == [(2, 3), (6, 7)]

    def test_zero_top_k_returns_empty(self) -> None:
        tok = _FakeTokenizer([(1, (0, 1))])
        assert top_k_surprising_spans("a", tok, lambda _: 1.0, top_k=0) == []

    def test_empty_source_returns_empty(self) -> None:
        tok = _FakeTokenizer([])
        assert top_k_surprising_spans("", tok, lambda _: 1.0, top_k=5) == []

    def test_meaningful_filter_drops_pieces(self) -> None:
        source = "abcde"
        pairs = [(1, (0, 2)), (2, (2, 4))]
        tok = _FakeTokenizer(pairs)
        not_meaningful: Callable[[int], bool] = lambda i: i != 2  # noqa: E731
        spans = top_k_surprising_spans(
            source, tok, lambda _: 1.0, top_k=5, is_meaningful=not_meaningful
        )
        assert spans == [(0, 2)]


# ---------------------------------------------------------------------------
# surprising_identifiers — end-to-end on a realistic UnixCoder span layout
# ---------------------------------------------------------------------------


class TestSurprisingIdentifiers:
    def test_end_to_end(self) -> None:
        source = "import mongoose from 'mongoose'"
        # UnixCoder-style: 'mongo' + 'ose' for both occurrences
        pairs = [
            (10, (0, 6)),  # 'import'
            (11, (7, 12)),  # 'mongo'
            (12, (12, 15)),  # 'ose'
            (13, (16, 20)),  # 'from'
            (14, (22, 27)),  # 'mongo'  (inside the string)
            (15, (27, 30)),  # 'ose'
        ]
        # Score 'mongo' / 'ose' high; everything else low.
        score = {11: 9.0, 12: 9.0, 14: 9.0, 15: 9.0, 10: 0.0, 13: 0.0}
        tok = _FakeTokenizer(pairs)
        result = surprising_identifiers(
            source, tok, lambda i: score.get(i, 0.0), top_k=4, max_identifiers=10
        )
        # Both expansions reconstruct to 'mongoose'; deduped to one.
        assert result == ["mongoose"]

    def test_max_identifiers_caps(self) -> None:
        source = "alpha bravo charlie delta echo"
        pairs = [
            (1, (0, 5)),
            (2, (6, 11)),
            (3, (12, 19)),
            (4, (20, 25)),
            (5, (26, 30)),
        ]
        tok = _FakeTokenizer(pairs)
        out = surprising_identifiers(source, tok, lambda _: 1.0, top_k=5, max_identifiers=2)
        assert len(out) == 2

    @pytest.mark.parametrize("k", [0, -1])
    def test_zero_or_negative_top_k_is_empty(self, k: int) -> None:
        tok = _FakeTokenizer([(1, (0, 5))])
        out = surprising_identifiers("alpha", tok, lambda _: 1.0, top_k=k, max_identifiers=10)
        assert out == []
