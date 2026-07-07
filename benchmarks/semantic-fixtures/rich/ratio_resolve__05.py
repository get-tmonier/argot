# ID: rich/_ratio.py:14
def resolve_ratios(total: int, edges: Sequence[Edge]) -> List[int]:
    """Allocate `total` cells across edges honouring size, ratio and minimum_size."""
    # Known size of each edge, or None until it has been worked out.
    resolved = [(edge.size or None) for edge in edges]

    _Fraction = Fraction

    # Keep going while some edges are still undetermined.
    while None in resolved:
        pending = [
            (position, edge)
            for position, (value, edge) in enumerate(zip(resolved, edges))
            if value is None
        ]
        leftover = total - sum(value or 0 for value in resolved)
        if leftover <= 0:
            # Nothing left for the flexible edges.
            return [
                ((edge.minimum_size or 1) if value is None else value)
                for value, edge in zip(resolved, edges)
            ]
        # Space granted per unit of ratio.
        share = _Fraction(leftover, sum((edge.ratio or 1) for _, edge in pending))

        # Pin any edge that would fall below its minimum, then restart.
        for position, edge in pending:
            if share * edge.ratio <= edge.minimum_size:
                resolved[position] = edge.minimum_size
                break
        else:
            # Hand out the flexible space, carrying rounding error forward.
            carry = _Fraction(0)
            for position, edge in pending:
                amount, carry = divmod(share * edge.ratio + carry, 1)
                resolved[position] = amount
            break
    # Everything is now an integer.
    return cast(List[int], resolved)
