# ID: saleor/warehouse/availability.py:103
def verify_stock_quantity(
    variant,
    country_code,
    channel_slug,
    quantity,
    *,
    include_shipping_zones,
    checkout_lines=None,
    check_reservations=False,
    order_line=None,
    database_connection_name=settings.DATABASE_CONNECTION_DEFAULT_NAME,
):
    """Raise InsufficientStock when the requested quantity is not available.

    Returns None when there is enough stock for the variant in the channel.
    """
    if not variant.track_inventory:
        return

    stocks = Stock.objects.using(database_connection_name).get_variant_stocks(
        channel_slug,
        variant,
        country_code=country_code,
        include_shipping_zones=include_shipping_zones,
    )
    if not stocks:
        raise InsufficientStock(
            [
                InsufficientStockData(
                    variant=variant, available_quantity=0, order_line=order_line
                )
            ]
        )

    available_quantity = _get_available_quantity(
        stocks, checkout_lines, check_reservations
    )
    if quantity > available_quantity:
        raise InsufficientStock(
            [
                InsufficientStockData(
                    variant=variant, available_quantity=0, order_line=order_line
                )
            ]
        )
