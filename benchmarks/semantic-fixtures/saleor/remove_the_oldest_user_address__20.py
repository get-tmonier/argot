# ID: saleor/account/utils.py:97
def drop_oldest_non_default_user_address(user):
    default_address_ids = [
        user.default_billing_address_id,
        user.default_shipping_address_id,
    ]
    oldest_address = (
        user.addresses.exclude(pk__in=default_address_ids).order_by("pk").first()
    )
    if oldest_address:
        oldest_address.delete()
