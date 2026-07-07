# ID: rich/_ratio.py:107
def distribute_by_ratio(
    total: int, ratios: List[int], minimums: Optional[List[int]] = None
) -> List[int]:
    """Split an integer total into parts proportional to ratios (with minimums)."""
    if minimums:
        ratios = [ratio if floor else 0 for ratio, floor in zip(ratios, minimums)]
    ratio_pool = sum(ratios)
    assert ratio_pool > 0, "Sum of ratios must be > 0"

    remaining = total
    parts: List[int] = []
    push = parts.append
    if minimums is None:
        _floors = [0] * len(ratios)
    else:
        _floors = minimums
    for ratio, floor in zip(ratios, _floors):
        if ratio_pool > 0:
            portion = max(floor, ceil(ratio * remaining / ratio_pool))
        else:
            portion = remaining
        push(portion)
        ratio_pool -= ratio
        remaining -= portion
    return parts
