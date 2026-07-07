# ID: rich/color.py:513
def to_system(self, system: ColorSystem) -> "Color":
    """Convert this color down to a color system with fewer colors."""

    if self.type in (ColorType.DEFAULT, system):
        return self
    # Truecolor -> 8-bit color
    if system == ColorSystem.EIGHT_BIT and self.system == ColorSystem.TRUECOLOR:
        assert self.triplet is not None
        _hue, lightness, saturation = rgb_to_hls(*self.triplet.normalized)
        # Low saturation is treated as grayscale.
        if saturation < 0.15:
            gray = round(lightness * 25.0)
            if gray == 0:
                color_number = 16
            elif gray == 25:
                color_number = 231
            else:
                color_number = 231 + gray
            return Color(self.name, ColorType.EIGHT_BIT, number=color_number)

        red, green, blue = self.triplet
        six_red = red / 95 if red < 95 else 1 + (red - 95) / 40
        six_green = green / 95 if green < 95 else 1 + (green - 95) / 40
        six_blue = blue / 95 if blue < 95 else 1 + (blue - 95) / 40

        color_number = (
            16 + 36 * round(six_red) + 6 * round(six_green) + round(six_blue)
        )
        return Color(self.name, ColorType.EIGHT_BIT, number=color_number)

    # Truecolor or 8-bit -> standard
    elif system == ColorSystem.STANDARD:
        if self.system == ColorSystem.TRUECOLOR:
            assert self.triplet is not None
            triplet = self.triplet
        else:  # self.system == ColorSystem.EIGHT_BIT
            assert self.number is not None
            triplet = ColorTriplet(*EIGHT_BIT_PALETTE[self.number])

        color_number = STANDARD_PALETTE.match(triplet)
        return Color(self.name, ColorType.STANDARD, number=color_number)

    elif system == ColorSystem.WINDOWS:
        if self.system == ColorSystem.TRUECOLOR:
            assert self.triplet is not None
            triplet = self.triplet
        else:  # self.system == ColorSystem.EIGHT_BIT
            assert self.number is not None
            if self.number < 16:
                return Color(self.name, ColorType.WINDOWS, number=self.number)
            triplet = ColorTriplet(*EIGHT_BIT_PALETTE[self.number])

        color_number = WINDOWS_PALETTE.match(triplet)
        return Color(self.name, ColorType.WINDOWS, number=color_number)

    return self
