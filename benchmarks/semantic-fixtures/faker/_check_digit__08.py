# ID: faker/providers/isbn/isbn.py:30
def _check_digit(self) -> str:
    """Calculate the check digit for an ISBN-13 (alternating weights 1 and 3)."""
    weights = (1 if i % 2 == 0 else 3 for i in range(12))
    parts = [self.ean, self.group, self.registrant, self.publication]
    body = "".join(part for part in parts if part is not None)
    remainder = sum(int(ch) * w for ch, w in zip(body, weights)) % 10
    difference = 10 - remainder
    digit = 0 if difference == 10 else difference
    return str(digit)
