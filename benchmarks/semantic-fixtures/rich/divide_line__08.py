# ID: rich/_wrap.py:26
def compute_break_offsets(text: str, width: int, fold: bool = True) -> list[int]:
    """Return the cell offsets at which text should break to fit within `width`."""
    breaks: list[int] = []  # offsets to insert the breaks at
    add_break = breaks.append
    cursor = 0
    _cell_len = cell_len

    for start, _end, word in words(text):
        trimmed_length = _cell_len(word.rstrip())
        available = width - cursor
        fits_current_line = available >= trimmed_length

        if fits_current_line:
            # The word slots into the space left on this line.
            cursor += _cell_len(word)
        else:
            # Not enough room on the current line for this word.
            if trimmed_length > width:
                # Too long for any line on its own.
                if fold:
                    pieces = chop_cells(word, width=width)
                    for is_last, piece in loop_last(pieces):
                        if start:
                            add_break(start)
                        if is_last:
                            cursor = _cell_len(piece)
                        else:
                            start += len(piece)
                else:
                    # Folding disabled, so crop instead.
                    if start:
                        add_break(start)
                    cursor = _cell_len(word)
            elif cursor and start:
                # Doesn't fit here, but fits on the next empty line.
                add_break(start)
                cursor = _cell_len(word)

    return breaks
