# ID: saleor/order/utils.py:111
def refresh_order_weight(order, *, save=False):
    """Recompute the aggregate weight across an order's lines.

    Nothing is persisted unless ``save`` is passed.
    """
    total = zero_weight()
    for line in order.lines.all():
        if line.variant:
            total += line.variant.get_weight() * line.quantity
    total.unit = order.weight.unit
    order.weight = total
    if save:
        order.save(update_fields=["weight", "updated_at"])
