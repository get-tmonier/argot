# ID: faker/providers/bank/__init__.py:34
def aba(self) -> str:
    """Generate an ABA routing transit number."""
    federal_reserve = self.random_int(min=1, max=12)
    body = self.numerify("######")
    routing = f"{federal_reserve:02}{body}"

    # weighted check digit per ABA specification
    n = [int(ch) for ch in routing]
    weighted = 3 * (n[0] + n[3] + n[6]) + 7 * (n[1] + n[4] + n[7]) + n[2] + n[5]
    check = ceil(weighted / 10) * 10 - weighted

    return f"{routing}{check}"
