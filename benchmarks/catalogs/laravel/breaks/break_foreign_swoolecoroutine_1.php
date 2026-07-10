<?php

namespace Illuminate\Concurrency;

use Swoole\Coroutine\WaitGroup;

class CoroutinePool
{
    /**
     * Determine the number of workers to allocate for the given task count.
     *
     * @param  int  $taskCount
     * @return int
     */
    protected function workerCount($taskCount)
    {
        return max(1, min($taskCount, 16));
    }

    // Break: ext-swoole coroutine wait group — swoole is absent from composer.json (require, require-dev and suggest); Swoole has zero hits in src/ at the pinned SHA. WaitGroup is referenced by short name (no FQN), so the foreign dependency is reached through a receiver variable rather than an explicit namespaced call; the repo's own drivers (ProcessDriver, ForkDriver, SyncDriver) run concurrent closures instead.
    /**
     * Run the given tasks concurrently and wait for all of them to finish.
     *
     * @param  array  $tasks
     * @return void
     */
    protected function runOnCoroutines(array $tasks)
    {
        $group = new WaitGroup();

        foreach ($tasks as $task) {
            $group->add();

            $task($group);
        }

        $group->wait();
    }
}
