# ID: wagtail/coreutils.py:97
def escaped_slugify(value):
    """Slugify like Django, but escape un-ASCIIfiable letters (e.g. Cyrillic) instead of dropping them so the result never comes out empty."""
    value = force_str(value)

    # Decompose accented Latin characters so the accent modifiers can be stripped,
    # leaving a clean ASCII base character behind.
    decomposed = unicodedata.normalize("NFKD", value)

    # Same regex Django's slugify uses to drop anything that isn't letter-like,
    # an underscore or a hyphen.
    stripped = SLUGIFY_RE.sub("", decomposed)

    # Escape any surviving non-ASCII characters as codes like 'u0421' rather than
    # losing them entirely.
    escaped = stripped.encode("ascii", "backslashreplace").decode("ascii")

    # Final pass through slugify handles whitespace and removes the escape backslashes.
    return slugify(escaped)
