# ID: saleor/payment/utils.py:562
def finalize_gateway_transaction(transaction, payment):
    updated_fields: list[str] = []

    if not transaction.is_success or transaction.already_processed:
        if updated_fields:
            payment.save(update_fields=updated_fields)
        return

    if transaction.action_required:
        payment.to_confirm = True
        updated_fields.append("to_confirm")
        payment.save(update_fields=updated_fields)
        return

    # When the gateway no longer requires an action, the payment does not
    # need confirmation either.
    if payment.to_confirm:
        payment.to_confirm = False
        updated_fields.append("to_confirm")

    update_payment_charge_status(payment, transaction, updated_fields)
