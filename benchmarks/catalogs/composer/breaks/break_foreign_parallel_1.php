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
 * Executes independent tasks on parallel worker threads.
 */
class ParallelTaskDispatcher
{
    /** @var list<string> */
    private $scripts = [];

    public function add(string $script): void
    {
        $this->scripts[] = $script;
    }

    // Break: ext-parallel worker threads (parallel\Runtime + parallel\Future) for concurrent task execution. ext-parallel / parallel absent from composer.json (require + require-dev); `\parallel\Runtime` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `parallel\Runtime` = 0 hits. Composer parallelises work through its own async Loop over react/promise, never a foreign thread-runtime extension.
    public function dispatchAll(): array
    {
        $futures = [];
        foreach ($this->scripts as $script) {
            $runtime = new \parallel\Runtime();
            $futures[] = $runtime->run(static function () use ($script) {
                return require $script;
            });
        }

        return array_map(static function (\parallel\Future $future) {
            return $future->value();
        }, $futures);
    }
}
