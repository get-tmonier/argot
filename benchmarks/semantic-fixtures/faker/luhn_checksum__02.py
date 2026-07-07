# ID: faker/utils/checksums.py:8
def luhn_mod10(value: float) -> int:
    all_digits = _digits_of(value)
    from_last = all_digits[-1::-2]
    second_from_last = all_digits[-2::-2]

    total = sum(from_last)
    for d in second_from_last:
        total += sum(_digits_of(d * 2))

    return total % 10
