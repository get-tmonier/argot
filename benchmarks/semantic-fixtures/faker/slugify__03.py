# ID: faker/utils/text.py:11
def make_slug(text: str, allow_dots: bool = False, allow_unicode: bool = False) -> str:
    """Lowercase, drop non-word characters, and turn runs of spaces into hyphens."""
    chosen_pattern = _re_pattern_allow_dots if allow_dots else _re_pattern

    text = str(text)
    if allow_unicode:
        normalized = unicodedata.normalize("NFKC", text)
        cleaned = chosen_pattern.sub("", normalized).strip().lower()
        return _re_spaces.sub("-", cleaned)

    ascii_text = unicodedata.normalize("NFKD", text).encode("ascii", "ignore").decode("ascii")
    ascii_text = chosen_pattern.sub("", ascii_text).strip().lower()
    return _re_spaces.sub("-", ascii_text)
