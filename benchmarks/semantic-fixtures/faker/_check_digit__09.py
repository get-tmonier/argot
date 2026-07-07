# ID: faker/providers/isbn/isbn.py:63
def _check_digit(self) -> str:
    """Calculate the check digit for an ISBN-10 (weights 1..9, modulo 11)."""
    weights = range(1, 10)
    parts = [self.group, self.registrant, self.publication]
    body = "".join(part for part in parts if part is not None)
    remainder = sum(int(ch) * w for ch, w in zip(body, weights)) % 11
    digit = "X" if remainder == 10 else str(remainder)
    return str(digit)
