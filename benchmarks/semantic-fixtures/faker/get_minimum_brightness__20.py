# ID: faker/providers/color/color.py:278
def get_minimum_brightness(self, h: int, s: int) -> int:
    """Linearly interpolate the minimum allowed brightness for a hue/saturation pair."""
    lower_bounds = self.get_color_info(h)["lower_bounds"]

    for i in range(len(lower_bounds) - 1):
        s1, v1 = lower_bounds[i]
        s2, v2 = lower_bounds[i + 1]

        if s1 <= s <= s2:
            slope = (v2 - v1) / (s2 - s1)
            intercept = v1 - slope * s1
            return int(slope * s + intercept)

    return 0
