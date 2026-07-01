from __future__ import annotations
import math
from collections import Counter


def tokenize(values):
    counts = Counter()
    for v in values:
        counts[v] += 1
    return counts


def entropy(counts):
    total = sum(counts.values())
    return -sum((c / total) * math.log(c / total) for c in counts.values())
