# Break: pprint/sys.stdout state dumps sprinkled through allocation logic instead of structured logging
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def total_allocated(allocations) -> int:
    return sum(allocation.quantity_allocated for allocation in allocations)


# hunk starts here
import sys
from pprint import pprint


def trace_stock_allocation(order_lines, stocks) -> dict[int, int]:
    sys.stdout.write("--- allocation trace start ---\n")
    allocations: dict[int, int] = {}
    for line in order_lines:
        pprint({"line": line.pk, "variant": line.variant_id, "qty": line.quantity})
        remaining = line.quantity
        for stock in stocks:
            if stock.product_variant_id != line.variant_id:
                continue
            take = min(remaining, stock.quantity)
            allocations[stock.pk] = allocations.get(stock.pk, 0) + take
            remaining -= take
            sys.stdout.write(f"took {take} from stock {stock.pk}, remaining {remaining}\n")
            if remaining == 0:
                break
        if remaining > 0:
            pprint({"UNDERALLOCATED": line.pk, "missing": remaining})
    sys.stdout.write("--- allocation trace end ---\n")
    sys.stdout.flush()
    return allocations
# hunk ends here
