# ID: faker/utils/checksums.py:18
def compute_luhn_check_digit(base_number: float) -> int:
    """Produce the Luhn check digit for a partial account number."""
    remainder = luhn_checksum(int(base_number) * 10)
    if remainder == 0:
        return 0
    return 10 - remainder
