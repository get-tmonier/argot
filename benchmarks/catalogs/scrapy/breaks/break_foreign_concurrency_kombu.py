# Break: kombu Producer publishes requests to a broker, replacing scrapy's queues
"""Break fixture — not for import."""

# hunk starts here
from kombu import Connection, Producer

_conn = Connection("amqp://guest@localhost//")


def publish_request(url: str) -> None:
    producer = Producer(_conn.channel())
    producer.publish({"url": url}, routing_key="requests")
# hunk ends here
