# Break: dramatiq actor + .send() runs snippet work off-process instead of wagtail's in-request flow
"""Break fixture — not for import."""
from __future__ import annotations

from django.contrib.contenttypes.models import ContentType


# Decoy — idiomatic wagtail snippet model resolution, NOT inside the hunk range
def resolve_snippet_model(app_label: str, model_name: str):
    return ContentType.objects.get(app_label=app_label, model=model_name).model_class()


# hunk starts here
import dramatiq


@dramatiq.actor(max_retries=3, time_limit=60000)
def reindex_snippet(app_label: str, model_name: str, pk: int) -> None:
    model = ContentType.objects.get(app_label=app_label, model=model_name).model_class()
    instance = model.objects.get(pk=pk)
    instance.save()


def enqueue_bulk_reindex(app_label: str, model_name: str, pks: list[int]) -> None:
    for pk in pks:
        reindex_snippet.send(app_label, model_name, pk)


@dramatiq.actor(queue_name="snippets.delete")
def purge_snippet(app_label: str, model_name: str, pk: int) -> None:
    model = ContentType.objects.get(app_label=app_label, model=model_name).model_class()
    model.objects.filter(pk=pk).delete()
# hunk ends here
