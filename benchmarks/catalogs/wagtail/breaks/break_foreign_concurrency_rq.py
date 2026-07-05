# Break: rq (Redis Queue) offloads workflow email instead of wagtail's sync send
"""Break fixture — not for import."""
from __future__ import annotations

from django.core.mail import send_mail as django_send_mail


# Decoy — idiomatic wagtail synchronous mail send, NOT inside the hunk range
def notify_now(subject: str, body: str, recipients: list[str]) -> int:
    return django_send_mail(subject, body, None, recipients)


# hunk starts here
from rq import Queue
from rq.job import Retry

_queue = Queue("wagtail-mail")


def _deliver(subject: str, body: str, recipients: list[str]) -> int:
    return django_send_mail(subject, body, None, recipients)


def enqueue_notification(subject: str, body: str, recipients: list[str]) -> str:
    job = _queue.enqueue(_deliver, subject, body, recipients, retry=Retry(max=3))
    return job.id


def enqueue_batch(messages: list[tuple[str, str, list[str]]]) -> list[str]:
    return [_queue.enqueue(_deliver, s, b, r).id for s, b, r in messages]
# hunk ends here
