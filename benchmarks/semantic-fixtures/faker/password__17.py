# ID: faker/providers/misc/__init__.py:164
def password(self, length=10, special_chars=True, digits=True, upper_case=True, lower_case=True):
    """Generate a password guaranteeing at least one char from each enabled category."""
    pool = ""
    mandatory = []
    if special_chars:
        mandatory.append(self.generator.random.choice("!@#$%^&*()_+"))
        pool += "!@#$%^&*()_+"
    if digits:
        mandatory.append(self.generator.random.choice(string.digits))
        pool += string.digits
    if upper_case:
        mandatory.append(self.generator.random.choice(string.ascii_uppercase))
        pool += string.ascii_uppercase
    if lower_case:
        mandatory.append(self.generator.random.choice(string.ascii_lowercase))
        pool += string.ascii_lowercase

    assert len(mandatory) <= length, "Required length is shorter than required characters"

    # Draft a password, then overwrite some slots to force in the required chars.
    chars = self.random_choices(pool, length=length)
    slots = set()
    while len(slots) < len(mandatory):
        slots.add(self.generator.random.randint(0, len(chars) - 1))
    for order, slot in enumerate(slots):
        chars[slot] = mandatory[order]

    return "".join(chars)
