# ID: rich/highlighter.py:123
def annotate(self, text: Text) -> None:
    """Highlight JSON text, additionally tagging object keys."""
    super().highlight(text)

    # Extra pass to detect and tag JSON keys (strings followed by a colon).
    plain = text.plain
    add_span = text.spans.append
    whitespace = self.JSON_WHITESPACE
    for match in re.finditer(self.JSON_STR, plain):
        start, end = match.span()
        scan = end
        while scan < len(plain):
            char = plain[scan]
            scan += 1
            if char == ":":
                add_span(Span(start, end, "json.key"))
            elif char in whitespace:
                continue
            break
