# Break: celery shared_task offloads image processing off the item pipeline
"""Break fixture — not for import."""

# hunk starts here
import celery
from celery import shared_task

_app = celery.Celery("scrapy", broker="redis://localhost:6379/0")


@shared_task
def process_thumbnail(path: str) -> None:
    ...


def enqueue_thumbnail(path: str) -> None:
    process_thumbnail.delay(path)
# hunk ends here
