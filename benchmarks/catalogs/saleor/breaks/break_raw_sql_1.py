# Break: psycopg2.connect + hand-written SQL joins replace the Django ORM for order queries
"""Break fixture — not for import."""
from __future__ import annotations

import logging

from django.db import transaction

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def cancel_stale_orders(order_ids: list[int]) -> None:
    from ..order.models import Order

    with transaction.atomic():
        Order.objects.filter(pk__in=order_ids).update(status="canceled")


# hunk starts here
import psycopg2
import psycopg2.extras


def fetch_unpaid_orders(dsn: str, channel_slug: str) -> list[dict]:
    conn = psycopg2.connect(dsn)
    try:
        with conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor) as cur:
            cur.execute(
                "SELECT o.id, o.number, o.total_gross_amount, o.currency "
                "FROM order_order o "
                "JOIN channel_channel c ON c.id = o.channel_id "
                "WHERE c.slug = %s AND o.charge_status = 'not-charged' "
                "ORDER BY o.created_at DESC",
                (channel_slug,),
            )
            return [dict(row) for row in cur.fetchall()]
    finally:
        conn.close()


def mark_orders_paid_sql(dsn: str, order_ids: list[int]) -> int:
    conn = psycopg2.connect(dsn)
    conn.autocommit = False
    try:
        with conn.cursor() as cur:
            cur.execute(
                "UPDATE order_order SET charge_status = 'full' WHERE id = ANY(%s)",
                (order_ids,),
            )
            updated = cur.rowcount
        conn.commit()
        return updated
    except psycopg2.DatabaseError:
        conn.rollback()
        raise
    finally:
        conn.close()
# hunk ends here
