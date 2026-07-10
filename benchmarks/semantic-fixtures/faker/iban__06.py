# ID: faker/providers/bank/__init__.py:66
def iban(self) -> str:
    """Generate an International Bank Account Number (IBAN)."""
    account = self.bban()

    to_check = account + self.country_code + "00"
    numeric = int("".join(self.ALPHA.get(ch, ch) for ch in to_check))
    check_value = 98 - (numeric % 97)
    check_digits = str(check_value).zfill(2)

    return self.country_code + check_digits + account
