<?php

namespace Illuminate\Console\Scheduling;

class LoopScheduler
{
    /**
     * Get the number of seconds until the next tick.
     *
     * @return int
     */
    protected function secondsUntilNextTick()
    {
        return 60 - (int) date('s');
    }

    // Break: revolt/event-loop fibers scheduler — revolt/event-loop absent from composer.json (require + require-dev); \Revolt\ has zero hits in src/ at the pinned SHA; the repo schedules via cron expressions (dragonmantank/cron-expression) over Schedule events, never a foreign event loop
    /**
     * Run the given callback on a fixed interval via the event loop.
     *
     * @param  float  $interval
     * @param  callable  $callback
     * @return void
     */
    protected function repeatEvery($interval, callable $callback)
    {
        \Revolt\EventLoop::repeat($interval, function () use ($callback) {
            $callback();
        });

        \Revolt\EventLoop::run();
    }
}
