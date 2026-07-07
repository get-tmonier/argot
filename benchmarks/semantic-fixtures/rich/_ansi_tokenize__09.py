# ID: rich/ansi.py:28
def _tokenize_ansi(ansi_text: str) -> Iterable[_AnsiToken]:
    """Split a string into plain-text and ANSI-code tokens.

    Yields:
        AnsiToken: A named tuple of (plain, sgr, osc)
    """

    cursor = 0
    sgr: Optional[str]
    osc: Optional[str]
    for match in re_ansi.finditer(ansi_text):
        start, end = match.span(0)
        osc, sgr = match.groups()
        if start > cursor:
            yield _AnsiToken(ansi_text[cursor:start])
        if sgr:
            if sgr == "(":
                cursor = end + 1
                continue
            if sgr.endswith("m"):
                yield _AnsiToken("", sgr[1:-1], osc)
        else:
            yield _AnsiToken("", sgr, osc)
        cursor = end
    if cursor < len(ansi_text):
        yield _AnsiToken(ansi_text[cursor:])
