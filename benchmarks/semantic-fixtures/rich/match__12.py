# ID: rich/palette.py:45
def closest(self, color: Tuple[int, int, int]) -> int:
    """Find the palette index whose color is nearest to the given color.

    Args:
        color (Tuple[int, int, int]): RGB components in range 0 > 255.

    Returns:
        int: Index of the closest matching color.
    """
    target_red, target_green, target_blue = color
    _sqrt = sqrt
    lookup = self._colors.__getitem__

    def color_distance(index: int) -> float:
        """Weighted RGB distance from the target to a palette color."""
        other_red, other_green, other_blue = lookup(index)
        red_mean = (target_red + other_red) // 2
        delta_red = target_red - other_red
        delta_green = target_green - other_green
        delta_blue = target_blue - other_blue
        return _sqrt(
            (((512 + red_mean) * delta_red * delta_red) >> 8)
            + 4 * delta_green * delta_green
            + (((767 - red_mean) * delta_blue * delta_blue) >> 8)
        )

    nearest_index = min(range(len(self._colors)), key=color_distance)
    return nearest_index
