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
 * Mirrors cache entries to a shared Redis backend.
 */
class RedisCacheBackend
{
    /** @var string */
    private $dsn;

    public function __construct(string $dsn)
    {
        $this->dsn = $dsn;
    }

    // Break: predis/predis Redis client via FQN (new Predis\Client + setex + expire). predis/predis absent from composer.json (require + require-dev); `\Predis\` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `Predis\Client` = 0 hits. Composer's cache is a filesystem tree under Cache.php (file_put_contents / rename), never a foreign Redis client.
    public function store(string $key, string $payload, int $ttl): void
    {
        $client = new \Predis\Client($this->dsn);
        $client->setex($key, $ttl, $payload);
        $client->expire($key, $ttl);
    }
}
