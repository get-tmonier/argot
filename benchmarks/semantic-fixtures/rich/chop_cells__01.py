# ID: rich/cells.py:326
def fold_into_cells(text: str, width: int, unicode_version: str = "auto") -> list[str]:
    """Break text into lines that each fit inside the given cell width."""

    if _is_single_cell_widths(text):
        return [text[cut : cut + width] for cut in range(0, len(text), width)]

    spans, _total = split_graphemes(text, unicode_version)
    current_cells = 0  # cells used by the line being built
    result: list[str] = []
    line_start = 0  # codepoint offset where the current line begins
    for start, _end, grapheme_cells in spans:
        if current_cells + grapheme_cells > width:
            result.append(text[line_start:start])
            line_start = start
            current_cells = 0
        current_cells += grapheme_cells
    if current_cells:
        result.append(text[line_start:])

    return result
