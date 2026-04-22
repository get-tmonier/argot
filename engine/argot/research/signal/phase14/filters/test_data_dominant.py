# engine/argot/research/signal/phase14/filters/test_data_dominant.py
"""Tests for is_data_dominant — structural heuristic, no I/O."""

from __future__ import annotations

from argot.research.signal.phase14.filters.data_dominant import is_data_dominant

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

# A file that is almost entirely top-level tuple/list assignments (>65% data).
_PURE_DATA_FILE = """\
CITIES = (
    "city_alpha",
    "city_beta",
    "city_gamma",
    "city_delta",
    "city_epsilon",
    "city_zeta",
    "city_eta",
    "city_theta",
    "city_iota",
    "city_kappa",
)
STREETS = [
    "street_one",
    "street_two",
    "street_three",
    "street_four",
    "street_five",
    "street_six",
    "street_seven",
    "street_eight",
    "street_nine",
    "street_ten",
]
DISTRICTS = (
    "district_a",
    "district_b",
    "district_c",
    "district_d",
    "district_e",
    "district_f",
    "district_g",
    "district_h",
)


def locale() -> str:
    return "xx_XX"
"""

# Simulate a dense unicode / locale table file (>65% data).
_UNICODE_TABLE_FILE = """\
UNICODE_TABLE = [
    (0x0041, "LATIN CAPITAL LETTER A"),
    (0x0042, "LATIN CAPITAL LETTER B"),
    (0x0043, "LATIN CAPITAL LETTER C"),
    (0x0044, "LATIN CAPITAL LETTER D"),
    (0x0045, "LATIN CAPITAL LETTER E"),
    (0x0046, "LATIN CAPITAL LETTER F"),
    (0x0047, "LATIN CAPITAL LETTER G"),
    (0x0048, "LATIN CAPITAL LETTER H"),
    (0x0049, "LATIN CAPITAL LETTER I"),
    (0x004A, "LATIN CAPITAL LETTER J"),
    (0x004B, "LATIN CAPITAL LETTER K"),
    (0x004C, "LATIN CAPITAL LETTER L"),
    (0x004D, "LATIN CAPITAL LETTER M"),
    (0x004E, "LATIN CAPITAL LETTER N"),
    (0x004F, "LATIN CAPITAL LETTER O"),
    (0x0050, "LATIN CAPITAL LETTER P"),
    (0x0051, "LATIN CAPITAL LETTER Q"),
    (0x0052, "LATIN CAPITAL LETTER R"),
    (0x0053, "LATIN CAPITAL LETTER S"),
    (0x0054, "LATIN CAPITAL LETTER T"),
    (0x0055, "LATIN CAPITAL LETTER U"),
    (0x0056, "LATIN CAPITAL LETTER V"),
    (0x0057, "LATIN CAPITAL LETTER W"),
    (0x0058, "LATIN CAPITAL LETTER X"),
    (0x0059, "LATIN CAPITAL LETTER Y"),
    (0x005A, "LATIN CAPITAL LETTER Z"),
]
"""

# A normal application module with a constant dict + multiple functions (should NOT be flagged).
_NORMAL_MODULE = """\
from __future__ import annotations

DEFAULTS = {"key": "value", "other": 42}


def process(x: str) -> str:
    result = DEFAULTS.get(x, "")
    cleaned = result.strip()
    return cleaned


def validate(x: str) -> bool:
    if not x:
        return False
    trimmed = x.strip()
    if len(trimmed) < 2:
        return False
    return trimmed.isidentifier()


class Handler:
    def run(self, item: str) -> bool:
        val = DEFAULTS.get(item)
        return val is not None

    def process_batch(self, items: list[str]) -> list[bool]:
        return [self.run(item) for item in items]
"""

# An "openapi/models.py"-style hybrid: class bodies have literal defaults, but
# most of the file is class/function definitions (should NOT be flagged).
_OPENAPI_HYBRID = """\
from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class Address:
    street: str = ""
    city: str = ""
    country: str = ""
    postal_code: str = ""
    extra: list[str] = field(default_factory=list)

    def format(self) -> str:
        parts = [self.street, self.city, self.country]
        return ", ".join(p for p in parts if p)

    def is_complete(self) -> bool:
        return bool(self.street and self.city and self.country)


@dataclass
class Person:
    name: str = ""
    age: int = 0
    addresses: list[Address] = field(default_factory=list)
    tags: list[str] = field(default_factory=list)

    def primary_address(self) -> Address | None:
        return self.addresses[0] if self.addresses else None

    def add_address(self, addr: Address) -> None:
        self.addresses.append(addr)


def build_person(name: str, age: int) -> Person:
    p = Person(name=name, age=age)
    default_addr = Address(city="Unknown")
    p.add_address(default_addr)
    return p
"""


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


def test_pure_data_file_detected() -> None:
    assert is_data_dominant(_PURE_DATA_FILE) is True


def test_unicode_table_file_detected() -> None:
    assert is_data_dominant(_UNICODE_TABLE_FILE) is True


def test_empty_string_returns_false() -> None:
    assert is_data_dominant("") is False


def test_whitespace_only_returns_false() -> None:
    assert is_data_dominant("   \n\n  \n") is False


def test_normal_module_with_embedded_constants_not_flagged() -> None:
    assert is_data_dominant(_NORMAL_MODULE) is False


def test_openapi_models_hybrid_not_flagged() -> None:
    assert is_data_dominant(_OPENAPI_HYBRID) is False


def test_threshold_parameter_respected() -> None:
    # _PURE_DATA_FILE is >65% data — tightening threshold to 0.99 should flip to False.
    assert is_data_dominant(_PURE_DATA_FILE, threshold=0.65) is True
    assert is_data_dominant(_PURE_DATA_FILE, threshold=0.99) is False


def test_syntax_error_returns_false() -> None:
    malformed = "def foo(\n    x: int\n    y: str\nreturn x\n"
    assert is_data_dominant(malformed) is False


def test_single_assignment_non_literal_not_flagged() -> None:
    src = "x = some_function()\n"
    assert is_data_dominant(src) is False


def test_dict_assignment_counted_as_data() -> None:
    # A file that is 100% a single dict assignment should be flagged.
    src = "MAPPING = {\n" + "\n".join(f'    "k{i}": {i},' for i in range(40)) + "\n}\n"
    assert is_data_dominant(src) is True


def test_class_body_data_assignments_detected() -> None:
    # Locale-style provider file: data lives in class body, not at module level.
    src = (
        "from typing import Tuple\n"
        "from ..base import Provider as BaseProvider\n"
        "\n"
        "class Provider(BaseProvider):\n"
        '    """Address provider."""\n'
        "\n"
        "    cities: Tuple[str, ...] = (\n"
        + "\n".join(f'        "city_{i}",' for i in range(60))
        + "\n    )\n"
        "    streets: Tuple[str, ...] = (\n"
        + "\n".join(f'        "street_{i}",' for i in range(60))
        + "\n    )\n"
        "\n"
        "    def fake_city(self) -> str:\n"
        "        return self.random_element(self.cities)\n"
    )
    assert is_data_dominant(src) is True


def test_class_with_balanced_methods_not_flagged() -> None:
    # A class with a small constant table but mostly methods should not be flagged.
    src = (
        "class Handler:\n"
        "    CODES = [1, 2, 3]\n"
        "\n"
        + "\n".join(
            f"    def method_{i}(self, x: int) -> int:\n        return x + {i}\n"
            for i in range(15)
        )
    )
    assert is_data_dominant(src) is False
