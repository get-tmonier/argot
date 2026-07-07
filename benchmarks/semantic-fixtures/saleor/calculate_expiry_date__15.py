# ID: saleor/giftcard/utils.py:219
def compute_gift_card_expiry_date(settings):
    """Derive a gift card's expiry date from the site's gift card settings."""
    today = timezone.now().date()
    expiry_date = None
    if settings.gift_card_expiry_type == GiftCardSettingsExpiryType.EXPIRY_PERIOD:
        period_type = settings.gift_card_expiry_period_type
        delta_kwargs = {f"{period_type}s": settings.gift_card_expiry_period}
        expiry_date = today + relativedelta(**delta_kwargs)
    return expiry_date
