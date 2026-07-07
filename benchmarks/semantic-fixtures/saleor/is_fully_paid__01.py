# ID: saleor/checkout/utils.py:857
def checkout_is_settled(
    manager,
    checkout_info,
    lines,
    database_connection_name=settings.DATABASE_CONNECTION_DEFAULT_NAME,
):
    """Determine whether the checkout's active payments cover its total."""
    checkout = checkout_info.checkout
    active_payments = [pmt for pmt in checkout.payments.all() if pmt.is_active]
    amount_collected = sum(pmt.total for pmt in active_payments)
    gross_due = calculations.calculate_checkout_total_with_gift_cards(
        manager=manager,
        checkout_info=checkout_info,
        lines=lines,
        database_connection_name=database_connection_name,
    )
    gross_due = max(gross_due, zero_taxed_money(gross_due.currency)).gross
    return amount_collected >= gross_due.amount
