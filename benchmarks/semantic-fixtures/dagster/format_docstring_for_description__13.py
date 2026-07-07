# ID: python_modules/dagster/dagster/_core/decorator_utils.py:125
def extract_description_from_docstring(fn):
    if fn.__doc__ is None:
        return None

    docstring = fn.__doc__
    if len(docstring) > 0 and docstring[0].isspace():
        return textwrap.dedent(docstring).strip()

    first_newline_pos = docstring.find("\n")
    if first_newline_pos == -1:
        return docstring
    return (
        docstring[: first_newline_pos + 1]
        + textwrap.dedent(docstring[first_newline_pos + 1 :])
    ).strip()
