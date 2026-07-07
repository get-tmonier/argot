# ID: wagtail/images/rect.py:103
def constrain_within(self, other):
    """Shift this rect (returning a new Rect) so it lies completely inside the `other` rect."""
    other = Rect(*other)
    result = self.clone()

    if result.left < other.left:
        result.right -= result.left - other.left
        result.left = other.left

    if result.top < other.top:
        result.bottom -= result.top - other.top
        result.top = other.top

    if result.right > other.right:
        result.left -= result.right - other.right
        result.right = other.right

    if result.bottom > other.bottom:
        result.top -= result.bottom - other.bottom
        result.bottom = other.bottom

    return result
