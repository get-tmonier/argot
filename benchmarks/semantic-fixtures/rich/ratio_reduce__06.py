# ID: rich/_ratio.py:75
def reduce_by_ratio(
    total: int, ratios: List[int], maximums: List[int], values: List[int]
) -> List[int]:
    """Subtract a total from values, split proportionally and capped by maximums."""
    ratios = [ratio if cap else 0 for ratio, cap in zip(ratios, maximums)]
    ratio_pool = sum(ratios)
    if not ratio_pool:
        return values[:]
    remaining = total
    reduced: List[int] = []
    push = reduced.append
    for ratio, maximum, value in zip(ratios, maximums, values):
        if ratio and ratio_pool > 0:
            taken = min(maximum, round(ratio * remaining / ratio_pool))
            push(value - taken)
            remaining -= taken
            ratio_pool -= ratio
        else:
            push(value)
    return reduced
