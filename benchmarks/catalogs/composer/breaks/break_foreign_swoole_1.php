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
 * Runs shell commands inside Swoole coroutines.
 */
class CoroutineProcessRunner
{
    /** @var list<string> */
    private $commands = [];

    public function enqueue(string $command): void
    {
        $this->commands[] = $command;
    }

    // Break: Swoole coroutine runtime (Coroutine\run + Coroutine::create + Coroutine\System::exec). ext-swoole / swoole absent from composer.json (require + require-dev); `\Swoole\` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `Swoole\Coroutine` = 0 hits. Composer runs external processes through Symfony Process via its own ProcessExecutor, never a foreign coroutine runtime.
    public function runAll(): void
    {
        \Swoole\Coroutine\run(function (): void {
            foreach ($this->commands as $command) {
                \Swoole\Coroutine::create(function () use ($command): void {
                    \Swoole\Coroutine\System::exec($command);
                });
            }
        });
    }
}
