# ID: faker/providers/python/__init__.py:255
def _safe_random_int(self, min_value, max_value, positive):
    original_min = min_value
    original_max = max_value

    if min_value is None:
        min_value = max_value - self.random_int()
    if max_value is None:
        max_value = min_value + self.random_int()
    if positive:
        min_value = max(min_value, 0)

    if min_value == max_value:
        return self._safe_random_int(original_min, original_max, positive)

    min_value = int(min_value)
    max_value = int(max_value - 1)
    if max_value < min_value:
        max_value += 1
    return self.random_int(min_value, max_value)
