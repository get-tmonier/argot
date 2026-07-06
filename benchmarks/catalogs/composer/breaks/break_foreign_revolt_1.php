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

use Revolt\EventLoop;

/**
 * Drives periodic progress ticks off a cooperative event loop.
 */
class EventLoopScheduler
{
    /** @var float */
    private $interval;

    public function __construct(float $interval)
    {
        $this->interval = $interval;
    }

    // Break: revolt/event-loop cooperative runtime reached through a submodule import (use Revolt\EventLoop; EventLoop::repeat / EventLoop::run); the foreign dependency is not an FQN in call position. revolt/event-loop absent from composer.json (require + require-dev); `Revolt\EventLoop` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `Revolt\EventLoop` = 0 hits. Composer coordinates concurrent work through its own Util\Loop over react/promise, never a foreign event-loop runtime.
    public function run(callable $onTick): void
    {
        EventLoop::repeat($this->interval, static function () use ($onTick) {
            $onTick();
        });
        EventLoop::run();
    }
}
