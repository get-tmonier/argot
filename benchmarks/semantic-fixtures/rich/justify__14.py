# ID: rich/containers.py:111
def align_lines(
    self,
    console: "Console",
    width: int,
    justify: "JustifyMethod" = "left",
    overflow: "OverflowMethod" = "fold",
) -> None:
    """Justify and overflow the stored lines to a given cell width, in place."""
    from .text import Text

    if justify == "left":
        for line in self._lines:
            line.truncate(width, overflow=overflow, pad=True)
    elif justify == "center":
        for line in self._lines:
            line.rstrip()
            line.truncate(width, overflow=overflow)
            line.pad_left((width - cell_len(line.plain)) // 2)
            line.pad_right(width - cell_len(line.plain))
    elif justify == "right":
        for line in self._lines:
            line.rstrip()
            line.truncate(width, overflow=overflow)
            line.pad_left(width - cell_len(line.plain))
    elif justify == "full":
        for row_index, line in enumerate(self._lines):
            if row_index == len(self._lines) - 1:
                break
            line_words = line.split(" ")
            used_width = sum(cell_len(word.plain) for word in line_words)
            gap_count = len(line_words) - 1
            gap_sizes = [1 for _ in range(gap_count)]
            pointer = 0
            if gap_sizes:
                while used_width + gap_count < width:
                    gap_sizes[len(gap_sizes) - pointer - 1] += 1
                    gap_count += 1
                    pointer = (pointer + 1) % len(gap_sizes)
            pieces: List[Text] = []
            for pointer, (word, next_word) in enumerate(
                zip_longest(line_words, line_words[1:])
            ):
                pieces.append(word)
                if pointer < len(gap_sizes):
                    style = word.get_style_at_offset(console, -1)
                    next_style = next_word.get_style_at_offset(console, 0)
                    gap_style = style if style == next_style else line.style
                    pieces.append(Text(" " * gap_sizes[pointer], style=gap_style))
            self[row_index] = Text("").join(pieces)
