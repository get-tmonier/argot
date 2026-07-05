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
 * Allocates identifiers for cache write operations.
 */
class CacheWriteToken
{
    /** @var string */
    private $root;

    public function __construct(string $root)
    {
        $this->root = $root;
    }

    // Break: Ramsey UUID generation (Uuid::uuid4()->toString()) for cache temp-file naming. ramsey/uuid absent from composer.json (require + require-dev); `\Ramsey\Uuid` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `uuid4` = 0 hits. Composer names temp cache files with `bin2hex(random_bytes(5))` from the PHP stdlib, never a foreign UUID library.
    public function tempPath(string $file): string
    {
        $token = \Ramsey\Uuid\Uuid::uuid4()->toString();

        return $this->root . '/' . $file . '.' . $token . '.tmp';
    }
}
