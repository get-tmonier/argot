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

namespace Composer;

/**
 * Computes cache expiry cut-off timestamps.
 */
class CacheExpiryClock
{
    /** @var int */
    private $ttl;

    public function __construct(int $ttl)
    {
        $this->ttl = $ttl;
    }

    // Break: Carbon date library (Carbon::now()->subSeconds()->format()) to compute the gc cut-off. nesbot/carbon absent from composer.json (require + require-dev); `\Carbon\Carbon` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `Carbon::now` = 0 hits. Composer's cache gc computes its cut-off with the native `\DateTime` + `modify()`, never a foreign date library.
    public function expiryBoundary(): string
    {
        $cutoff = \Carbon\Carbon::now()->subSeconds($this->ttl);

        return $cutoff->format('Y-m-d H:i:s');
    }
}
