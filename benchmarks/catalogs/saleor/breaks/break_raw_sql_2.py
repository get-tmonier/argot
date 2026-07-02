# Break: django connection.cursor() with hand-built SQL for domain reads instead of ORM querysets
"""Break fixture — not for import."""
from __future__ import annotations

import logging
from decimal import Decimal

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def get_active_rule_ids(rules) -> list[int]:
    return list(rules.filter(reward_value__gt=Decimal(0)).values_list("id", flat=True))


# hunk starts here
from django.db import connection


def fetch_rules_for_channel(channel_id: int) -> list[tuple]:
    sql = (
        "SELECT r.id, r.name, r.reward_value, r.reward_value_type "
        "FROM discount_promotionrule r "
        "JOIN discount_promotionrule_channels rc ON rc.promotionrule_id = r.id "
        "WHERE rc.channel_id = %s AND r.reward_value IS NOT NULL"
    )
    with connection.cursor() as cursor:
        cursor.execute(sql, [channel_id])
        return cursor.fetchall()


def count_gift_rules_by_promotion() -> dict[int, int]:
    with connection.cursor() as cursor:
        cursor.execute(
            "SELECT promotion_id, COUNT(*) FROM discount_promotionrule "
            "WHERE reward_type = 'gift' GROUP BY promotion_id"
        )
        rows = cursor.fetchall()
    return {promotion_id: count for promotion_id, count in rows}
# hunk ends here
