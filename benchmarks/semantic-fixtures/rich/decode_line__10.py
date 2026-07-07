# ID: rich/ansi.py:138
def parse_line(self, line: str) -> Text:
    """Turn a single line of ANSI-coded terminal output into styled Text."""
    from_ansi = Color.from_ansi
    from_rgb = Color.from_rgb
    _Style = Style
    text = Text()
    append = text.append
    line = line.rsplit("\r", 1)[-1]
    for plain_text, sgr, osc in _ansi_tokenize(line):
        if plain_text:
            append(plain_text, self.style or None)
        elif osc is not None:
            if osc.startswith("8;"):
                _params, semicolon, link = osc[2:].partition(";")
                if semicolon:
                    self.style = self.style.update_link(link or None)
        elif sgr is not None:
            # Translate to semicolon-separated codes, ignoring invalid ones.
            codes = [
                min(255, int(raw) if raw else 0)
                for raw in sgr.split(";")
                if raw.isdigit() or raw == ""
            ]
            code_iter = iter(codes)
            for code in code_iter:
                if code == 0:
                    # reset
                    self.style = _Style.null()
                elif code in SGR_STYLE_MAP:
                    # styles
                    self.style += _Style.parse(SGR_STYLE_MAP[code])
                elif code == 38:
                    # Foreground
                    with suppress(StopIteration):
                        color_mode = next(code_iter)
                        if color_mode == 5:
                            self.style += _Style.from_color(
                                from_ansi(next(code_iter))
                            )
                        elif color_mode == 2:
                            self.style += _Style.from_color(
                                from_rgb(
                                    next(code_iter),
                                    next(code_iter),
                                    next(code_iter),
                                )
                            )
                elif code == 48:
                    # Background
                    with suppress(StopIteration):
                        color_mode = next(code_iter)
                        if color_mode == 5:
                            self.style += _Style.from_color(
                                None, from_ansi(next(code_iter))
                            )
                        elif color_mode == 2:
                            self.style += _Style.from_color(
                                None,
                                from_rgb(
                                    next(code_iter),
                                    next(code_iter),
                                    next(code_iter),
                                ),
                            )

    return text
