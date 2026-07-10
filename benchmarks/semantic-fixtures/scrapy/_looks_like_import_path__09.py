# ID: scrapy/utils/python.py:340

def _resembles_import_path(value):
    """Return True when *value* has the shape of a dotted Python import path."""
    if not value:
        return False
    if any(ch.isspace() for ch in value):
        return False
    permitted = set(
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_."
    )
    if any(ch not in permitted for ch in value):
        return False
    if value.startswith(".") or value.endswith("."):
        return False
    segments = value.split(".")
    if any(seg == "" for seg in segments):
        return False
    return all(seg.isidentifier() for seg in segments)
