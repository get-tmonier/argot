# ID: rich/color.py:485
def ansi_codes(self, foreground: bool = True) -> Tuple[str, ...]:
    """Build the ANSI escape parameters that select this color."""
    color_kind = self.type
    if color_kind == ColorType.DEFAULT:
        return ("39" if foreground else "49",)

    elif color_kind == ColorType.WINDOWS:
        number = self.number
        assert number is not None
        base_fore, base_back = (30, 40) if number < 8 else (82, 92)
        return (str(base_fore + number if foreground else base_back + number),)

    elif color_kind == ColorType.STANDARD:
        number = self.number
        assert number is not None
        base_fore, base_back = (30, 40) if number < 8 else (82, 92)
        return (str(base_fore + number if foreground else base_back + number),)

    elif color_kind == ColorType.EIGHT_BIT:
        assert self.number is not None
        return ("38" if foreground else "48", "5", str(self.number))

    else:  # color_kind == ColorType.TRUECOLOR
        assert self.triplet is not None
        red, green, blue = self.triplet
        return ("38" if foreground else "48", "2", str(red), str(green), str(blue))
