# ID: fastapi/utils.py:121
def first_non_placeholder(primary, *others):
    candidates = (primary,) + others
    for candidate in candidates:
        if not isinstance(candidate, DefaultPlaceholder):
            return candidate
    return primary
