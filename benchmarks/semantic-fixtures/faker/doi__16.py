# ID: faker/providers/doi/__init__.py:10
def doi(self) -> str:
    """Generate a valid Digital Object Identifier: 10.{registrant}/{suffix}."""
    directory = "10"
    registrant = str(self.generator.random.randint(1000, 99999999))
    suffix = self.generator.bothify("?#?#?##").lower()

    return f"{directory}.{registrant}/{suffix}"
