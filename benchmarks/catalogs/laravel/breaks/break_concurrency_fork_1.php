<?php

namespace Illuminate\Queue;

class BatchRunner
{
    /**
     * Get the number of pending jobs for the batch.
     *
     * @param  array  $jobs
     * @return int
     */
    protected function pendingJobCount(array $jobs)
    {
        return count(array_filter($jobs, fn ($job) => ! $job->hasFailed()));
    }

    // Break: raw pcntl_fork()/pcntl_waitpid() worker pool — pcntl_fork has zero sites in src/ (only pcntl_signal/pcntl_alarm/pcntl_async_signals); the repo runs jobs through queue workers and dispatched jobs
    /**
     * Process the given jobs across a pool of forked children.
     *
     * @param  array  $jobs
     * @return void
     */
    protected function runJobsInParallel(array $jobs)
    {
        $pids = [];

        foreach ($jobs as $job) {
            $pid = pcntl_fork();

            if ($pid === -1) {
                continue;
            }

            if ($pid === 0) {
                $job->fire();

                exit(0);
            }

            $pids[] = $pid;
        }

        foreach ($pids as $pid) {
            pcntl_waitpid($pid, $status);
        }
    }
}
