# ID: faker/providers/ssn/en_US/__init__.py:142
def invalid_ssn(self) -> str:
    """Generate a random US SSN that is invalid and also not a valid ITIN."""
    itin_group_numbers = [
        70, 71, 72, 73, 74, 75, 76, 77, 78, 79,
        80, 81, 82, 83, 84, 85, 86, 87, 88,
        90, 91, 92, 94, 95, 96, 97, 98, 99,
    ]
    area = self.random_int(min=0, max=999)
    if area < 900 and area not in {666, 0}:
        coin = self.random_int(min=1, max=1000)
        if coin <= 500:
            group = 0
            serial = self.random_int(0, 9999)
        else:
            group = self.random_int(0, 99)
            serial = 0
    elif area in {666, 0}:
        group = self.random_int(0, 99)
        serial = self.random_int(0, 9999)
    else:
        group = self.random_element([x for x in range(0, 100) if x not in itin_group_numbers])
        serial = self.random_int(0, 9999)

    return f"{area:03d}-{group:02d}-{serial:04d}"
