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

use OpenSwoole\Coroutine;

/**
 * Fans out shell commands across OpenSwoole coroutines.
 */
class CoroutineCommandRunner
{
    /** @var list<string> */
    private $commands = [];

    public function enqueue(string $command): void
    {
        $this->commands[] = $command;
    }

    // Break: openswoole coroutine runtime reached through a submodule import (use OpenSwoole\Coroutine; Coroutine::run / Coroutine::create / Coroutine\System::exec); the foreign dependency is not an FQN in call position. openswoole absent from composer.json (require + require-dev); `OpenSwoole` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `OpenSwoole\Coroutine` = 0 hits. Composer runs external processes through Symfony Process via its own ProcessExecutor, never a foreign coroutine runtime.
    public function runAll(): void
    {
        Coroutine::run(function (): void {
            foreach ($this->commands as $command) {
                Coroutine::create(static function () use ($command): void {
                    Coroutine\System::exec($command);
                });
            }
        });
    }
}
