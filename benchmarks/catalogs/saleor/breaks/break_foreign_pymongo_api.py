# Break: MongoDB collection ops (aggregate/update/count) on a pymongo handle replace the Django ORM for order rollups
"""Break fixture — not for import."""

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style ORM helper, NOT inside the hunk range
def recent_order_ids(channel_slug: str) -> list[int]:
    from saleor.order.models import Order

    return list(
        Order.objects.filter(channel__slug=channel_slug).values_list("id", flat=True)
    )


# hunk starts here
def rollup_order_totals(db) -> tuple[dict, int]:
    pipeline = [
        {"$match": {"status": "fulfilled"}},
        {"$group": {"_id": "$channel_id", "total": {"$sum": "$total_gross_amount"}}},
    ]
    rows = db.orders.aggregate(pipeline)
    db.orders.update(
        {"channel_id": {"$exists": True}}, {"$set": {"rolled_up": True}}, multi=True
    )
    return {row["_id"]: row["total"] for row in rows}, db.orders.count()
# hunk ends here
