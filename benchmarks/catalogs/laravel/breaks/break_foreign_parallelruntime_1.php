<?php

namespace Illuminate\Concurrency;

class RuntimePool
{
    /**
     * Determine the maximum number of runtimes to keep warm.
     *
     * @param  int  $taskCount
     * @return int
     */
    protected function poolSize($taskCount)
    {
        return max(1, min($taskCount, 8));
    }

    // Break: ext-parallel runtime pool — ext-parallel is absent from composer.json (require, require-dev and suggest); \parallel\ has zero hits in src/ at the pinned SHA; the repo's own ProcessDriver forks worker processes through Illuminate\Process\Pool instead of a foreign runtime extension. Explicit foreign namespace \parallel\.
    /**
     * Run the given tasks on a pool of parallel runtimes and collect the results.
     *
     * @param  array  $tasks
     * @return array
     */
    protected function runOnRuntimes(array $tasks)
    {
        $results = [];

        foreach ($tasks as $key => $task) {
            $runtime = new \parallel\Runtime();

            $future = $runtime->run($task);

            $results[$key] = $future->value();
        }

        return $results;
    }
}
