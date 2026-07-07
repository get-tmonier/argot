# ID: saleor/giftcard/utils.py:276
def gift_card_has_expired(gift_card):
    """Return True when the gift card's expiry date is already in the past."""
    current_date = timezone.now().date()
    return bool(gift_card.expiry_date) and gift_card.expiry_date < current_date
