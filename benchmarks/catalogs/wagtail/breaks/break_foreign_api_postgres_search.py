# Break: django.contrib.postgres.search full-text expressions bypass the search backend
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.models import Page
from wagtail.search.backends import get_search_backend


# Decoy — idiomatic wagtail search-backend query, NOT inside the hunk range
def backend_search(query_string: str):
    return get_search_backend().search(query_string, Page.objects.live())


# hunk starts here
from django.contrib.postgres.search import (
    SearchQuery,
    SearchRank,
    SearchVector,
    TrigramSimilarity,
)


def rank_pages(query_string: str):
    vector = SearchVector("title", weight="A") + SearchVector("search_description", weight="B")
    query = SearchQuery(query_string, search_type="websearch")
    return (
        Page.objects.live()
        .annotate(rank=SearchRank(vector, query))
        .filter(rank__gte=0.1)
        .order_by("-rank")
    )


def fuzzy_titles(term: str):
    return (
        Page.objects.live()
        .annotate(similarity=TrigramSimilarity("title", term))
        .filter(similarity__gt=0.3)
        .order_by("-similarity")
    )
# hunk ends here
