# Break: joblib (aliased) fans daily-hits aggregation across worker processes
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.contrib.search_promotions.models import Query


# Decoy — idiomatic wagtail ORM query hit-count read, NOT inside the hunk range
def hit_total(query_string: str) -> int:
    query = Query.get(query_string)
    return query.hits


# hunk starts here
import joblib as _jl


def _recount(query_id: int) -> tuple[int, int]:
    query = Query.objects.get(pk=query_id)
    return query_id, query.daily_hits.count()


def parallel_recount(query_ids: list[int]) -> dict[int, int]:
    results = _jl.Parallel(n_jobs=4, backend="loky")(
        _jl.delayed(_recount)(qid) for qid in query_ids
    )
    return dict(results)
# hunk ends here
