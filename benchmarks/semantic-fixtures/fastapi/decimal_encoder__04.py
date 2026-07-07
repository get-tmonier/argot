# ID: fastapi/encoders.py:43
def encode_decimal(dec_value):
    """Encode a Decimal as int when it has no fractional exponent, otherwise as float."""
    exp = dec_value.as_tuple().exponent
    has_no_fraction = isinstance(exp, int) and exp >= 0
    if has_no_fraction:
        return int(dec_value)
    return float(dec_value)
