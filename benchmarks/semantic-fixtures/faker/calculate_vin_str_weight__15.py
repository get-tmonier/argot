# ID: faker/providers/automotive/__init__.py:10
def calculate_vin_str_weight(s: str, weight_factor: list) -> int:
    """Multiply each transliterated character by its positional weight and sum."""

    def _get_char_weight(c: str) -> int:
        # A=1..I=9, J=1..R=9, S=2..Z=9; digits map to themselves.
        if ord(c) <= 64:
            return int(c)
        if ord(c) <= 73:
            return ord(c) - 64
        if ord(c) <= 82:
            return ord(c) - 73
        return ord(c) - 81

    accumulator = 0
    for i, c in enumerate(s):
        if i < len(weight_factor):
            accumulator += _get_char_weight(c) * weight_factor[i]
    return accumulator
