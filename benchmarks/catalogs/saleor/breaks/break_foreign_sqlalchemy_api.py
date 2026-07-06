# Break: SQLAlchemy Core engine/query (module-qualified) replaces the Django ORM for order reporting
"""Break fixture — not for import."""
from __future__ import annotations

import logging

import sqlalchemy

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style ORM helper, NOT inside the hunk range
def recent_order_ids(channel_slug: str) -> list[int]:
    from saleor.order.models import Order

    return list(
        Order.objects.filter(channel__slug=channel_slug).values_list("id", flat=True)
    )


# hunk starts here
def order_totals_by_channel(dsn: str) -> list[tuple[str, float]]:
    engine = sqlalchemy.create_engine(dsn, pool_pre_ping=True)
    metadata = sqlalchemy.MetaData()
    orders = sqlalchemy.Table(
        "order_order",
        metadata,
        sqlalchemy.Column("id"),
        sqlalchemy.Column("channel_id"),
        sqlalchemy.Column("total_gross_amount"),
    )
    query = sqlalchemy.select(orders.c.channel_id, orders.c.total_gross_amount)
    with engine.connect() as conn:
        rows = conn.execute(query).fetchall()
    return [(str(row.channel_id), float(row.total_gross_amount)) for row in rows]
# hunk ends here
