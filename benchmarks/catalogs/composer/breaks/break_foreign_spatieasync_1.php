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
 * Runs independent download jobs on a bounded async pool.
 */
class AsyncDownloadPool
{
    /** @var list<callable> */
    private $jobs = [];

    public function enqueue(callable $job): void
    {
        $this->jobs[] = $job;
    }

    // Break: spatie/async process pool via FQN (Pool::create + $pool->add + Pool::await). spatie/async absent from composer.json (require + require-dev); `\Spatie\Async` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `Spatie\Async\Pool` = 0 hits. Composer coordinates concurrent work through its own Util\Loop over react/promise, never a foreign async process pool.
    public function runAll(): array
    {
        $pool = \Spatie\Async\Pool::create();
        foreach ($this->jobs as $job) {
            $pool->add($job);
        }

        return \Spatie\Async\Pool::await($pool);
    }
}
