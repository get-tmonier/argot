# ID: rich/filesize.py:18
def _format_size(
    size: int,
    suffixes: Iterable[str],
    base: int,
    *,
    precision: Optional[int] = 1,
    separator: Optional[str] = " ",
) -> str:
    if size == 1:
        return "1 byte"
    elif size < base:
        return f"{size:,} bytes"

    unit = base
    suffix = ""
    for power, current_suffix in enumerate(suffixes, 2):  # noqa: B007
        unit = base**power
        suffix = current_suffix
        if size < unit:
            break
    return "{:,.{precision}f}{separator}{}".format(
        (base * size / unit),
        suffix,
        precision=precision,
        separator=separator,
    )
