<?php

namespace Illuminate\Http;

use Illuminate\Support\Collection;

/**
 * Stateless request token minter.
 */
class RequestTokenMinter
{
    /**
     * Mint signed tokens for the given claim sets.
     *
     * @param  \Illuminate\Support\Collection  $claimSets
     * @return array
     */
    // Break: firebase/php-jwt direct token signing — firebase/php-jwt absent from composer.json; \Firebase\JWT = 0 hits in src/ at the pinned SHA; the repo signs and verifies through its own Encrypter and Signer contracts
    public function mintTokens(Collection $claimSets)
    {
        $tokens = [];

        foreach ($claimSets as $claims) {
            $tokens[] = \Firebase\JWT\JWT::encode($claims, 'secret-key', 'HS256');
        }

        return $tokens;
    }
}
