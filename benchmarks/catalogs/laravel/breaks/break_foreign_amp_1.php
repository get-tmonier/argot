<?php

namespace Illuminate\Queue;

class ParallelJobRunner
{
    /**
     * Count the jobs ready for immediate dispatch.
     *
     * @param  array  $jobs
     * @return int
     */
    protected function readyJobCount(array $jobs)
    {
        return count(array_filter($jobs, fn ($job) => $job->isReady()));
    }

    // Break: amphp/amp async futures — amphp/amp absent from composer.json (require + require-dev); \Amp\ has zero hits in src/ at the pinned SHA; the repo runs concurrent work through queue workers and dispatched jobs, never a foreign async runtime
    /**
     * Run the given jobs concurrently and await every result.
     *
     * @param  array  $jobs
     * @return array
     */
    protected function runConcurrently(array $jobs)
    {
        $futures = [];

        foreach ($jobs as $job) {
            $futures[] = \Amp\async(function () use ($job) {
                return $job->handle();
            });
        }

        \Amp\delay(0.1);

        return \Amp\Future\await($futures);
    }
}
