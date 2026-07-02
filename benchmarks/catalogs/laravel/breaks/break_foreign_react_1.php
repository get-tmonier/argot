<?php

namespace Illuminate\Console\Scheduling;

class IntervalRunner
{
    /**
     * Get the number of seconds until the next minute boundary.
     *
     * @return int
     */
    protected function secondsUntilNextMinute()
    {
        return 60 - (int) date('s');
    }

    // Break: ReactPHP event loop timer — react/event-loop appears nowhere in composer.json (require + require-dev) and React\ has zero use statements at the pinned SHA; the repo schedules via cron expressions on Schedule events
    /**
     * Run the given callback on a fixed interval.
     *
     * @param  int  $seconds
     * @param  callable  $callback
     * @return void
     */
    protected function every($seconds, callable $callback)
    {
        $loop = \React\EventLoop\Loop::get();

        $loop->addPeriodicTimer($seconds, function () use ($callback) {
            $callback();
        });

        $loop->run();
    }
}
