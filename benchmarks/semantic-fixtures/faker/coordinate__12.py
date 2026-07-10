# ID: faker/providers/geo/__init__.py:1005
def coordinate(self, center=None, radius=0.001):
    """Pick a decimal coordinate, optionally within `radius` of a center point."""
    if center is None:
        raw = self.generator.random.randint(-180000000, 180000000) / 1000000
        return Decimal(str(raw)).quantize(Decimal(".000001"))

    center = float(center)
    radius = float(radius)
    point = self.generator.random.uniform(center - radius, center + radius)
    return Decimal(str(point)).quantize(Decimal(".000001"))
