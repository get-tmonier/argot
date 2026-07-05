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
 * Mirrors a local artifact onto a remote filesystem abstraction.
 */
class RemoteArtifactMirror
{
    /** @var string */
    private $prefix;

    public function __construct(string $prefix)
    {
        $this->prefix = rtrim($prefix, '/');
    }

    // Break: HARD — league/flysystem filesystem reached only through a receiver whose leaf methods collide with composer's own attested vocabulary (write=207 calls/8 defs, read=51/3, has=5/1 across src at c6c9144f1b75). The foreign type `\League\Flysystem\FilesystemOperator` appears ONLY in the parameter type hint — no `use`, no FQN in call position — so the import and call-receiver stages have no foreign namespace to catch; only bpe surprise could fire. league/flysystem absent from composer.json (require + require-dev); `League\Flysystem` = 0 grep hits at c6c9144f1b75 (all *.php, src included). Composer manipulates files through its own Util\Filesystem (native rename / file_put_contents), never a foreign filesystem abstraction. Honest hard case: may miss.
    public function mirror(\League\Flysystem\FilesystemOperator $remote, string $path, string $contents): bool
    {
        $target = $this->prefix . '/' . ltrim($path, '/');
        $remote->write($target, $contents);
        if (!$remote->has($target)) {
            return false;
        }

        return $remote->read($target) === $contents;
    }
}
