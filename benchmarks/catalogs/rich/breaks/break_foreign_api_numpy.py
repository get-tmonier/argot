# Break: numpy array math computes bar geometry (foreign numeric API)
"""Break fixture — not for import."""
from __future__ import annotations


# Decoy — idiomatic pure-python ratio, NOT inside the hunk range
def simple_ratio(value: float, total: float, width: int) -> int:
    if total <= 0:
        return 0
    return int(width * value / total)


# hunk starts here
import numpy


def compute_bar_segments(values: list[float], width: int) -> list[int]:
    arr = numpy.array(values, dtype=numpy.float64)
    normalized = arr / arr.sum()
    widths = numpy.floor(normalized * width).astype(int)
    return widths.tolist()
# hunk ends here
