"""Unit tests for the shared layout helpers in :mod:`evidence.layout`.

The PRD's UX bundle (D6) is encoded as constants there; these tests pin
the wording / truncation behaviour so a future tweak surfaces as a test
diff rather than silently shipping. Every formatter ultimately depends on
these helpers so any drift here ripples to all three reasons at once.
"""

from __future__ import annotations

import pytest

from argot.scoring.evidence.layout import (
    COMMON_HERE_FLOOR,
    RARITY_DENOMINATOR_FLOOR,
    format_common_here_line,
    format_frequency,
    format_rarity,
    should_show_common_here,
    truncate_with_overflow,
)
from argot.scoring.evidence.types import CommonEntry, RarityStat


class TestTruncateWithOverflow:
    def test_below_cap_renders_all(self) -> None:
        assert truncate_with_overflow(["a", "b"], k=3) == "a, b"

    def test_at_cap_renders_all(self) -> None:
        assert truncate_with_overflow(["a", "b", "c"], k=3) == "a, b, c"

    def test_overflow_shows_more_suffix(self) -> None:
        assert truncate_with_overflow(["a", "b", "c", "d", "e"], k=3) == "a, b, c (+2 more)"

    def test_overflow_singular_still_uses_more(self) -> None:
        # PRD spec uses "(+N more)" without singular/plural switching
        assert truncate_with_overflow(["a", "b", "c", "d"], k=3) == "a, b, c (+1 more)"


class TestFormatFrequency:
    @pytest.mark.parametrize(
        ("count", "expected"),
        [
            (0, "0×"),
            (1, "1×"),
            (847, "847×"),
            (3_200, "3,200×"),
            (12_400, "12,400×"),
        ],
    )
    def test_format(self, count: int, expected: str) -> None:
        assert format_frequency(count) == expected


class TestShouldShowCommonHere:
    def test_empty_is_suppressed(self) -> None:
        assert should_show_common_here([]) is False

    def test_top_one_below_floor_suppressed(self) -> None:
        # Floor is 3; top-1=2 → suppress
        entries = [CommonEntry("a", 2), CommonEntry("b", 1)]
        assert should_show_common_here(entries) is False

    def test_top_one_at_floor_shown(self) -> None:
        entries = [CommonEntry("a", COMMON_HERE_FLOOR)]
        assert should_show_common_here(entries) is True

    def test_top_one_above_floor_shown(self) -> None:
        entries = [CommonEntry("a", 320)]
        assert should_show_common_here(entries) is True


class TestFormatRarity:
    def test_above_floor_shows_full_phrase(self) -> None:
        rarity = RarityStat(0, 12_400, "identifiers", "repo")
        assert format_rarity(rarity) == "0 of 12,400 identifiers in repo"

    def test_below_floor_falls_back_to_never_seen(self) -> None:
        rarity = RarityStat(0, RARITY_DENOMINATOR_FLOOR - 1, "identifiers", "repo")
        assert format_rarity(rarity) == "never seen in repo"

    def test_below_floor_uses_cluster_where(self) -> None:
        rarity = RarityStat(0, 12, "callees", "this cluster")
        assert format_rarity(rarity) == "never seen in this cluster"

    def test_above_floor_handles_cluster_phrase(self) -> None:
        rarity = RarityStat(0, 1247, "callees", "this cluster")
        assert format_rarity(rarity) == "0 of 1,247 callees in this cluster"

    def test_nonzero_flagged_count_renders(self) -> None:
        # The PRD notes flagged_count is "almost always 0" but should render
        # honestly when non-zero; pin that.
        rarity = RarityStat(2, 47, "module specifiers", "repo")
        assert format_rarity(rarity) == "2 of 47 module specifiers in repo"


class TestFormatCommonHereLine:
    def test_renders_with_frequency_suffix(self) -> None:
        entries = [
            CommonEntry("useEffect", 320),
            CommonEntry("fetch", 180),
            CommonEntry("render", 120),
        ]
        assert format_common_here_line(entries) == ("useEffect (320×), fetch (180×), render (120×)")

    def test_overflow_appended(self) -> None:
        entries = [
            CommonEntry("a", 10),
            CommonEntry("b", 9),
            CommonEntry("c", 8),
            CommonEntry("d", 7),
            CommonEntry("e", 6),
        ]
        # k=3 default; 5 - 3 = 2 more
        assert format_common_here_line(entries) == "a (10×), b (9×), c (8×) (+2 more)"

    def test_under_cap_no_overflow(self) -> None:
        entries = [CommonEntry("a", 10), CommonEntry("b", 9)]
        assert format_common_here_line(entries) == "a (10×), b (9×)"
