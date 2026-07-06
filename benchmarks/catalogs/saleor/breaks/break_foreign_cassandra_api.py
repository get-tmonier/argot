# Break: Cassandra CQL run through a driver session.execute() mimics the attested cursor.execute() raw-SQL idiom
"""Break fixture — not for import."""

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style stock helper, NOT inside the hunk range
def available_quantity(stock) -> int:
    return max(stock.quantity - stock.quantity_allocated, 0)


# hunk starts here
def record_stock_movements(session, movements) -> list[tuple[str, int]]:
    for move in movements:
        session.execute(
            "INSERT INTO stock_movements (sku, delta, ts) "
            "VALUES (%s, %s, toTimestamp(now()))",
            (move.sku, move.delta),
        )
    rows = session.execute(
        "SELECT sku, delta FROM stock_movements WHERE active = true ALLOW FILTERING"
    )
    return [(row.sku, row.delta) for row in rows]
# hunk ends here
