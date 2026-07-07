# ID: saleor/discount/utils/voucher.py:69
def register_voucher_usage(
    voucher,
    code,
    customer_email,
    increase_voucher_customer_usage=True,
):
    if voucher.usage_limit:
        increase_voucher_code_usage_value(code)
    if voucher.apply_once_per_customer and increase_voucher_customer_usage:
        add_voucher_usage_by_customer(code, customer_email)
    if voucher.single_use:
        deactivate_voucher_code(code)
