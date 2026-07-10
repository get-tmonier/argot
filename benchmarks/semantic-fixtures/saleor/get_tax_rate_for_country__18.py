# ID: saleor/tax/utils.py:362
def resolve_country_tax_rate(tax_class_country_rates, default_tax_rate, country_code):
    """Return the tax rate matching the country code, or the default.

    ``tax_class_country_rates`` is the collection of per-country rates belonging
    to a single tax class.
    """
    rate = default_tax_rate
    for country_rate in tax_class_country_rates:
        if country_rate.country == country_code:
            rate = country_rate.rate
    return rate
