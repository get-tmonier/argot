# Break: pika (imported in the hunk) opens a raw AMQP channel, replacing the Celery/kombu task dispatch
"""Break fixture — not for import."""

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def delivery_routing_key(event_type: str) -> str:
    return f"webhook.{event_type}"


# hunk starts here
import pika


def publish_webhook_event(host: str, routing_key: str, body: bytes) -> None:
    connection = pika.BlockingConnection(pika.ConnectionParameters(host=host))
    channel = connection.channel()
    channel.queue_declare(queue="webhooks", durable=True)
    channel.basic_publish(exchange="", routing_key=routing_key, body=body)
    connection.close()
# hunk ends here
