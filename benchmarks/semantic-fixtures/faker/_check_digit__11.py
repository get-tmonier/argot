# ID: faker/providers/sbn/sbn.py:26
def _check_digit(self) -> str:
    """Calculate the SBN-9 check digit (ISBN-style, but weights only run 1..8)."""
    weights = range(1, 9)
    parts = [self.registrant, self.publication]
    body = "".join(part for part in parts if part is not None)
    remainder = sum(int(ch) * w for ch, w in zip(body, weights)) % 11
    digit = "X" if remainder == 10 else str(remainder)
    return str(digit)
