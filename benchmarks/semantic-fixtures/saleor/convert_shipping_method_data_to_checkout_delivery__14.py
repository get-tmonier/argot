# ID: saleor/shipping/utils.py:91
def build_checkout_delivery_from_shipping_method(shipping_method_data, checkout):
    tax_class_details = {}
    if shipping_method_data.tax_class:
        tax_class_details = {
            "tax_class_id": shipping_method_data.tax_class.id,
            "tax_class_name": shipping_method_data.tax_class.name,
            "tax_class_metadata": shipping_method_data.tax_class.metadata,
            "tax_class_private_metadata": shipping_method_data.tax_class.private_metadata,
        }

    is_external = shipping_method_data.is_external
    built_in_id = int(shipping_method_data.id) if not is_external else None
    external_id = shipping_method_data.id if is_external else None

    return CheckoutDelivery(
        checkout=checkout,
        external_shipping_method_id=external_id,
        built_in_shipping_method_id=built_in_id,
        name=shipping_method_data.name or "",
        description=shipping_method_data.description,
        price_amount=shipping_method_data.price.amount,
        currency=shipping_method_data.price.currency,
        maximum_delivery_days=shipping_method_data.maximum_delivery_days,
        minimum_delivery_days=shipping_method_data.minimum_delivery_days,
        metadata=shipping_method_data.metadata,
        private_metadata=shipping_method_data.private_metadata,
        active=shipping_method_data.active,
        message=shipping_method_data.message,
        is_valid=True,
        is_external=is_external,
        **tax_class_details,
    )
