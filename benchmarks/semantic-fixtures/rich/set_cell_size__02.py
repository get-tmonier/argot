# ID: rich/cells.py:299
def fit_to_cells(text: str, total: int, unicode_version: str = "auto") -> str:
    """Pad with spaces or crop a string so it occupies exactly `total` cells."""

    if _is_single_cell_widths(text):
        plain_size = len(text)
        if plain_size < total:
            return text + " " * (total - plain_size)
        return text[:total]
    if total <= 0:
        return ""
    current_size = cell_len(text)
    if current_size == total:
        return text
    if current_size < total:
        return text + " " * (total - current_size)
    cropped, _remainder = _split_text(text, total, unicode_version)
    return cropped
