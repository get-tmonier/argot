# Break: joblib.Parallel fans rendering across processes (foreign concurrency lib)
"""Break fixture — not for import."""
from __future__ import annotations


# Decoy — idiomatic sequential render, NOT inside the hunk range
def render_all(items: list[str]) -> list[str]:
    return [item.strip() for item in items]


# hunk starts here
from joblib import Parallel, delayed


def render_parallel(items: list[str], workers: int = 4) -> list[str]:
    results = Parallel(n_jobs=workers)(
        delayed(str.strip)(item) for item in items
    )
    return list(results)
# hunk ends here
