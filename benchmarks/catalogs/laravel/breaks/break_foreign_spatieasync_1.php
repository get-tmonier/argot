<?php

namespace Illuminate\Queue;

class BatchJobRunner
{
    /**
     * Count the jobs that failed within the batch.
     *
     * @param  array  $results
     * @return int
     */
    protected function failedJobCount(array $results)
    {
        return count(array_filter($results, fn ($result) => $result === false));
    }

    // Break: spatie/async worker pool — spatie/async absent from composer.json (require + require-dev); \Spatie\Async\ has zero hits in src/ at the pinned SHA (only spatie/fork, a different package, is referenced as an optional "fork" concurrency driver); the repo runs concurrent work through queue workers and dispatched jobs, never this foreign async pool
    /**
     * Process the given jobs across an async worker pool and collect the results.
     *
     * @param  array  $jobs
     * @return array
     */
    protected function processAsync(array $jobs)
    {
        $pool = \Spatie\Async\Pool::create();

        foreach ($jobs as $job) {
            $pool->add(function () use ($job) {
                return $job->handle();
            });
        }

        return $pool->wait();
    }
}
