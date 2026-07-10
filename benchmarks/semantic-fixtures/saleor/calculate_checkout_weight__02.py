# ID: saleor/checkout/utils.py:929
def total_checkout_weight(lines):
    accumulated = zero_weight()
    for line_info in lines:
        variant = line_info.variant
        if variant:
            unit_weight = get_checkout_line_weight(line_info)
            accumulated += unit_weight * line_info.line.quantity
    return accumulated
