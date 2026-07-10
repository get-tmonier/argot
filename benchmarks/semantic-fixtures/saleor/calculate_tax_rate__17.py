# ID: saleor/tax/utils.py:350
def compute_tax_rate(price):
    """Derive the effective tax rate (as a fraction) from a TaxedMoney price.

    Both the net and gross components must be set for a non-zero result.
    """
    rate = Decimal("0.0")
    # A zero net or gross short-circuits to a zero rate.
    if not isinstance(price, Decimal) and all((price.gross, price.net)):
        rate = price.tax / price.net
    return rate
