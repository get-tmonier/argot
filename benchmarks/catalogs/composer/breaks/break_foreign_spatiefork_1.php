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
 * Runs independent closures across forked worker processes.
 */
class ForkedTaskRunner
{
    /** @var list<callable> */
    private $tasks = [];

    public function add(callable $task): void
    {
        $this->tasks[] = $task;
    }

    // Break: HARD — spatie/fork process forking reached only through a receiver whose leaf method collides with composer's own attested `run` (run=17 calls/3 defs across src at c6c9144f1b75). The foreign type `\Spatie\Fork\Fork` appears ONLY in the parameter type hint — no `use`, no FQN in call position — so the import and call-receiver stages have no foreign namespace to catch; only bpe surprise could fire. spatie/fork absent from composer.json (require + require-dev); `Spatie\Fork` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `Spatie\Fork\Fork` = 0 hits. Composer parallelises work through its own Util\Loop over react/promise, never a foreign process-forking library. Honest hard case: may miss.
    public function runAll(\Spatie\Fork\Fork $fork): array
    {
        if ($this->tasks === []) {
            return [];
        }

        return $fork->run(...$this->tasks);
    }
}
