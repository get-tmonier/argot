# ID: saleor/payment/utils.py:721
def decimal_to_minor_unit(value, currency):
    """Convert a decimal amount into the currency's smallest integer unit.

    e.g. Decimal("10.00") in USD becomes the string "1000".
    """
    normalized = quantize_price(value, currency=currency)
    precision = get_currency_precision(currency)
    scale = Decimal("10.0") ** precision
    scaled = normalized * scale
    return str(scaled.quantize(Decimal(1)))
