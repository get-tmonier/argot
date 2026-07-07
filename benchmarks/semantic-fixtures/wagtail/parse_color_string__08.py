# ID: wagtail/images/utils.py:58
def hex_to_rgb(color_string):
    """Parse a CSS 3- or 6-digit hex colour (no leading '#') into an (r, g, b) tuple. Raises ValueError for any other length."""
    if len(color_string) == 3:
        red = int(color_string[0], 16) * 17
        green = int(color_string[1], 16) * 17
        blue = int(color_string[2], 16) * 17
    elif len(color_string) == 6:
        red = int(color_string[0:2], 16)
        green = int(color_string[2:4], 16)
        blue = int(color_string[4:6], 16)
    else:
        raise ValueError("Color string must be either 3 or 6 hexadecimal digits long")

    return red, green, blue
