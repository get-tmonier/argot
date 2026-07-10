# ID: faker/providers/credit_card/__init__.py:172
def _generate_number(self, prefix: str, length: int) -> str:
    """Build a Luhn-valid credit card number from a prefix and target length."""
    number = prefix + "#" * (length - len(prefix) - 1)
    number = self.numerify(number)

    reverse = number[::-1]
    tot = 0
    pos = 0
    while pos < length - 1:
        tot += Provider.luhn_lookup[reverse[pos]]
        if pos != length - 2:
            tot += int(reverse[pos + 1])
        pos += 2

    check_digit = (10 - (tot % 10)) % 10
    return number + str(check_digit)
