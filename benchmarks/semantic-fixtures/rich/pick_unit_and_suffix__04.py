# ID: rich/filesize.py:43
def choose_unit_and_suffix(size: int, suffixes: List[str], base: int) -> Tuple[int, str]:
    """Select the unit magnitude and suffix appropriate for the given size."""
    unit = 1
    suffix = suffixes[0]
    for power, candidate_suffix in enumerate(suffixes):
        unit = base**power
        suffix = candidate_suffix
        if size < unit * base:
            break
    return unit, suffix
