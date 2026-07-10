# ID: saleor/order/utils.py:498
def apply_order_amount_to_gift_card(gift_card, total_price_left, balance_data):
    starting_balance = gift_card.current_balance
    if total_price_left < gift_card.current_balance:
        gift_card.current_balance = gift_card.current_balance - total_price_left
        total_price_left = zero_money(total_price_left.currency)
    else:
        total_price_left = total_price_left - gift_card.current_balance
        gift_card.current_balance_amount = 0
    balance_data.append((gift_card, starting_balance.amount))
    return total_price_left
