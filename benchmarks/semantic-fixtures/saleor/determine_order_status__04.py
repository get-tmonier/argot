# ID: saleor/order/utils.py:210
def resolve_order_status(
    total_quantity,
    quantity_fulfilled,
    quantity_returned,
    quantity_awaiting_approval,
):
    effective_fulfilled = quantity_fulfilled - quantity_awaiting_approval
    if effective_fulfilled <= 0:
        return OrderStatus.UNFULFILLED
    if 0 < quantity_returned < total_quantity:
        return OrderStatus.PARTIALLY_RETURNED
    if quantity_returned == total_quantity:
        return OrderStatus.RETURNED
    if effective_fulfilled < total_quantity:
        return OrderStatus.PARTIALLY_FULFILLED
    return OrderStatus.FULFILLED
