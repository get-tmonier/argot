<?php

namespace Illuminate\Http;

use phpseclib3\Crypt\RSA;

class RequestSigner
{
    /**
     * Build the canonical string for the given request payload.
     *
     * @param  array  $payload
     * @return string
     */
    protected function canonicalize(array $payload)
    {
        ksort($payload);

        return http_build_query($payload);
    }

    // Break: phpseclib3 RSA signing — phpseclib/phpseclib absent from composer.json (require + require-dev); phpseclib3 has zero hits in src/ at the pinned SHA. The key class is reached by short name and driven through a receiver variable; the repo signs and verifies through its own Encrypter and the ext-openssl primitives it already depends on.
    /**
     * Produce an RSA signature over the canonical request string.
     *
     * @param  array  $payload
     * @return string
     */
    protected function signRequest(array $payload)
    {
        $message = $this->canonicalize($payload);

        $privateKey = RSA::createKey(2048);

        $signature = $privateKey->withPadding(RSA::SIGNATURE_PSS)->sign($message);

        return base64_encode($signature);
    }
}
