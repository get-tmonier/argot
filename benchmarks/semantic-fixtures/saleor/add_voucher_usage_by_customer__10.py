# ID: saleor/discount/utils/voucher.py:112
def record_customer_voucher_use(code, customer_email):
    if not customer_email:
        raise NotApplicable("Unable to apply voucher as customer details are missing.")

    _, newly_created = VoucherCustomer.objects.get_or_create(
        voucher_code=code, customer_email=customer_email
    )
    if not newly_created:
        raise NotApplicable("This offer is only valid once per customer.")
