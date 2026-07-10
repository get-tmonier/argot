# ID: scrapy/utils/python.py:102

def reverse_regex_search(pattern, text, chunk_size=1024):
    """Search *text* for *pattern* from the end, reading it in reverse chunks.

    Returns the (start, end) span of the last match relative to the whole text,
    or None when nothing matches.
    """

    def _reverse_chunks():
        pos = len(text)
        while True:
            pos -= chunk_size * 1024
            if pos <= 0:
                break
            yield (text[pos:], pos)
        yield (text, 0)

    if isinstance(pattern, str):
        pattern = re.compile(pattern)

    for chunk, base in _reverse_chunks():
        found = list(pattern.finditer(chunk))
        if found:
            start, end = found[-1].span()
            return base + start, base + end
    return None
