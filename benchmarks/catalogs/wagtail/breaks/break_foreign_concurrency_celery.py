# Break: celery shared_task + .delay() offloads notification email instead of wagtail's sync send
"""Break fixture — not for import."""
from __future__ import annotations

from django.core.mail import send_mail as django_send_mail


# Decoy — idiomatic wagtail synchronous mail send, NOT inside the hunk range
def notify_now(subject: str, body: str, recipients: list[str]) -> int:
    return django_send_mail(subject, body, None, recipients)


# hunk starts here
import celery
from celery import shared_task


@shared_task(bind=True, max_retries=3)
def send_notification_email(self, subject: str, body: str, recipients: list[str]) -> None:
    try:
        django_send_mail(subject, body, None, recipients)
    except OSError as exc:
        raise self.retry(exc=exc, countdown=10)


def queue_notification(subject: str, body: str, recipients: list[str]) -> str:
    result = send_notification_email.delay(subject, body, recipients)
    return result.id


def broadcast(subject: str, body: str, groups: list[list[str]]) -> celery.group:
    job = celery.group(
        send_notification_email.s(subject, body, recipients) for recipients in groups
    )
    return job.apply_async()
# hunk ends here
