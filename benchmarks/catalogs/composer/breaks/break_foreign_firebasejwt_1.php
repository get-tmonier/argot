<?php declare(strict_types=1);

/*
 * This file is part of Composer.
 *
 * (c) Nils Adermann <naderman@naderman.de>
 *     Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

namespace Composer\Util;

/**
 * Mints signed bearer tokens for authenticated registry requests.
 */
class BearerTokenFactory
{
    /** @var string */
    private $secret;

    public function __construct(string $secret)
    {
        $this->secret = $secret;
    }

    // Break: firebase/php-jwt token signing via FQN (JWT::encode with an HS256 secret). firebase/php-jwt absent from composer.json (require + require-dev); `\Firebase\JWT` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `JWT::encode` = 0 hits. Composer authenticates registry requests through its own AuthHelper / IOInterface credential store, never a foreign JWT library.
    public function issue(string $origin, int $ttl): string
    {
        $claims = [
            'iss' => 'composer',
            'aud' => $origin,
            'exp' => time() + $ttl,
        ];

        return \Firebase\JWT\JWT::encode($claims, $this->secret, 'HS256');
    }
}
