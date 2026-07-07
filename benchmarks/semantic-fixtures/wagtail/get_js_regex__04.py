# ID: wagtail/coreutils.py:565
def python_regex_to_js(regex=None, base_js_flags="gu"):
    """Convert a Python regex (or pattern string) into a [pattern, flags] pair usable by JavaScript's new RegExp()."""
    if not regex:
        # new RegExp('') in JavaScript matches nothing, so mirror that with an empty list.
        return []

    if isinstance(regex, re.Pattern):
        py_flags = regex.flags
    else:
        regex = re.compile(regex)
        py_flags = regex.flags

    equivalents = {
        re.IGNORECASE: "i",
        re.MULTILINE: "m",
        re.DOTALL: "s",
    }

    if py_flags & re.LOCALE:
        raise ValueError("Python re.LOCALE flag is not supported in JavaScript.")
    if py_flags & re.VERBOSE:
        raise ValueError("Python re.VERBOSE flag is not supported in JavaScript.")

    mapped = [js_flag for py_flag, js_flag in equivalents.items() if py_flags & py_flag]

    # Merge with base flags, de-duplicate and sort for a stable order.
    final_flags = "".join(sorted(set(base_js_flags + "".join(mapped))))

    # Inline flag groups such as (?i) are invalid in JavaScript, so strip them out.
    cleaned_pattern = re.sub(r"(?i)(\(\?[a-z]+\))", "", regex.pattern)

    return [cleaned_pattern, final_flags]
