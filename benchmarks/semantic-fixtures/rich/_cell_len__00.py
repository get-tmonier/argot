# ID: rich/cells.py:113
def measure_cell_width(content: str, unicode_version: str) -> int:
    """Compute how many terminal cells a string occupies."""

    if _is_single_cell_widths(content):
        return len(content)

    zero_width_joiner = "‍"
    variation_selector = "️"
    if zero_width_joiner not in content and variation_selector not in content:
        # Nothing that changes glyph size, so just sum the individual widths.
        return sum(
            get_character_cell_size(glyph, unicode_version) for glyph in content
        )

    table = load_cell_table(unicode_version)
    running_width = 0
    previous_glyph: str | None = None

    modifiers = {zero_width_joiner, variation_selector}

    cursor = 0
    glyph_count = len(content)

    while cursor < glyph_count:
        glyph = content[cursor]
        if glyph in modifiers:
            if glyph == zero_width_joiner:
                cursor += 1
            elif previous_glyph:
                running_width += previous_glyph in table.narrow_to_wide
                previous_glyph = None
        else:
            if glyph_width := get_character_cell_size(glyph, unicode_version):
                previous_glyph = glyph
                running_width += glyph_width
        cursor += 1

    return running_width
