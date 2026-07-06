# Break: graphene GraphQL schema replaces wagtail's DRF-style API viewsets
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.models import Page


# Decoy — idiomatic wagtail ORM page lookup, NOT inside the hunk range
def live_pages(limit: int = 20):
    return Page.objects.live().public().order_by("-first_published_at")[:limit]


# hunk starts here
import graphene


class PageType(graphene.ObjectType):
    id = graphene.ID()
    title = graphene.String()
    slug = graphene.String()
    url_path = graphene.String()


class PageQuery(graphene.ObjectType):
    pages = graphene.List(PageType, limit=graphene.Int())

    def resolve_pages(self, info, limit=20):
        return [
            PageType(id=p.pk, title=p.title, slug=p.slug, url_path=p.url_path)
            for p in Page.objects.live()[:limit]
        ]


page_schema = graphene.Schema(query=PageQuery)
# hunk ends here
