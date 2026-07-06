# Break: pymongo MongoClient mirrors site events, bypassing the Django ORM
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.models import Site


# Decoy — idiomatic wagtail ORM site lookup, NOT inside the hunk range
def default_site():
    return Site.objects.filter(is_default_site=True).first()


# hunk starts here
from pymongo import MongoClient

_mongo = MongoClient("mongodb://localhost:27017")
_events = _mongo.wagtail.site_events


def record_site_event(site: Site, kind: str, payload: dict) -> str:
    doc = {"site_id": site.pk, "hostname": site.hostname, "kind": kind, "data": payload}
    result = _events.insert_one(doc)
    return str(result.inserted_id)


def recent_site_events(site: Site, limit: int = 50) -> list[dict]:
    cursor = _events.find({"site_id": site.pk}).sort("_id", -1).limit(limit)
    return list(cursor)
# hunk ends here
