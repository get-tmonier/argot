<?php

namespace Illuminate\Cache;

class PriceCache
{
    /**
     * Build the cache key for the given SKU.
     *
     * @param  string  $sku
     * @return string
     */
    protected function priceKey($sku)
    {
        return 'price:'.strtolower($sku);
    }

    // Break: moneyphp/money value objects — moneyphp/money absent from composer.json (require + require-dev); \Money\ has zero hits in src/ at the pinned SHA; the repo has no money value type and models amounts with brick/math and plain scalars instead
    /**
     * Compute the total price for the given line items.
     *
     * @param  array  $lineItems
     * @return \Money\Money
     */
    protected function totalFor(array $lineItems)
    {
        $total = new \Money\Money(0, new \Money\Currency('USD'));

        foreach ($lineItems as $item) {
            $amount = new \Money\Money($item['cents'], new \Money\Currency('USD'));
            $total = $total->add($amount);
        }

        return $total;
    }
}
