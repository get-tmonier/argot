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
 * Awaits a batch of pending job promises.
 */
class JobBatchAwaiter
{
    /** @var array<int, callable> */
    private $tasks = [];

    public function schedule(callable $task): void
    {
        $this->tasks[] = $task;
    }

    // Break: Amp async runtime (Amp\async + Amp\Future::await) to run the job batch concurrently. amphp/amp absent from composer.json (require + require-dev); `\Amp\` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `Amp\Future` = 0 hits. Composer coordinates concurrent work through its own Loop over react/promise, never a foreign async runtime.
    public function awaitAll(): array
    {
        $futures = [];
        foreach ($this->tasks as $task) {
            $futures[] = \Amp\async($task);
        }

        return \Amp\Future\await($futures);
    }
}
