# ID: faker/providers/automotive/__init__.py:54
def vin(self) -> str:
    """Generate a VIN with a valid check character."""
    allowed = "1234567890ABCDEFGHJKLMNPRSTUVWXYZ"  # I, O, Q are not permitted
    head = self.bothify("????????", letters=allowed)
    tail = self.bothify("????####", letters=allowed)

    head_weight = calculate_vin_str_weight(head, [8, 7, 6, 5, 4, 3, 2, 10])
    tail_weight = calculate_vin_str_weight(tail, [9, 8, 7, 6, 5, 4, 3, 2])
    remainder = (head_weight + tail_weight) % 11
    check_char = "X" if remainder == 10 else str(remainder)

    return head + check_char + tail
