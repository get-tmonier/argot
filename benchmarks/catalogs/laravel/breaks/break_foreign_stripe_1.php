<?php

namespace Illuminate\Http\Client;

use Illuminate\Support\Collection;

/**
 * Outbound payment dispatcher.
 */
class PaymentDispatcher
{
    /**
     * Dispatch the given charges to the payment gateway.
     *
     * @param  \Illuminate\Support\Collection  $charges
     * @return array
     */
    // Break: Stripe SDK direct payment-intent creation — stripe/stripe-php absent from composer.json; \Stripe\ = 0 hits in src/ at the pinned SHA; the repo issues all HTTP through its own Guzzle-backed client, never a vendor SDK
    public function dispatchCharges(Collection $charges)
    {
        $client = new \Stripe\StripeClient('sk_live_placeholder');

        $receipts = [];

        foreach ($charges as $charge) {
            $intent = $client->paymentIntents->create([
                'amount' => $charge['amount'],
                'currency' => 'usd',
            ]);
            $receipts[] = $intent->id;
        }

        return $receipts;
    }
}
