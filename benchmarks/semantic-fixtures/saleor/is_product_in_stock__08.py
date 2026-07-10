# ID: saleor/warehouse/availability.py:564
def product_has_available_stock(
    product,
    country_code,
    channel_slug,
    calculate_stocks_with_shipping_zones,
):
    """Return True if any variant of the product is available in the channel."""
    stocks = Stock.objects.get_product_stocks(
        channel_slug,
        product,
        country_code=country_code,
        include_shipping_zones=calculate_stocks_with_shipping_zones,
    ).annotate_available_quantity()
    return any(stocks.values_list("available_quantity", flat=True))
