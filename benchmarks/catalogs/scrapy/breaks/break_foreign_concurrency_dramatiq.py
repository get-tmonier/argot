# Break: dramatiq actor sends mail off the reactor, replacing scrapy's MailSender
"""Break fixture — not for import."""

# hunk starts here
import dramatiq


@dramatiq.actor(max_retries=3)
def send_report_email(to: str, body: str) -> None:
    ...


def enqueue_report(to: str, body: str) -> None:
    send_report_email.send(to, body)
# hunk ends here
