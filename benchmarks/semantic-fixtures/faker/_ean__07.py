# ID: faker/providers/barcode/__init__.py:20
def _ean(self, length: int = 13, prefixes: PrefixType = ()) -> str:
    if length not in (8, 13):
        raise AssertionError("length can only be 8 or 13")

    digits = [self.random_digit() for _ in range(length - 1)]

    if prefixes:
        chosen_prefix = self.random_element(prefixes)
        digits[: len(chosen_prefix)] = map(int, chosen_prefix)

    if length == 8:
        weights = [3, 1, 3, 1, 3, 1, 3]
    elif length == 13:
        weights = [1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3]

    weighted_sum = sum(d * w for d, w in zip(digits, weights))
    check_digit = (10 - weighted_sum % 10) % 10
    digits.append(check_digit)

    return "".join(str(d) for d in digits)
