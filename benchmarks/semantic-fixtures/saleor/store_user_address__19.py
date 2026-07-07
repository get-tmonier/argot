# ID: saleor/account/utils.py:58
def save_user_address(user, address, address_type, manager):
    """Persist an address in the user's address book, defaulting it if unset."""

    # Respect the per-user address cap; drop the new address silently if reached.
    if is_user_address_limit_reached(user):
        return

    address_data = address.as_data()

    stored = user.addresses.filter(**address_data).first()
    if stored is None:
        stored = user.addresses.create(**address_data)

    if address_type == AddressType.BILLING:
        if not user.default_billing_address:
            set_user_default_billing_address(user, stored)
    elif address_type == AddressType.SHIPPING:
        if not user.default_shipping_address:
            set_user_default_shipping_address(user, stored)
