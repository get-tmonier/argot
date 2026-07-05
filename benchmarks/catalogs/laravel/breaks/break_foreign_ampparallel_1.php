<?php

namespace Illuminate\Bus;

use Amp\Parallel\Worker\DefaultWorkerPool;
use Illuminate\Support\Collection;

class WorkerPoolDispatcher
{
    /**
     * Partition the batch into evenly sized chunks.
     *
     * @param  \Illuminate\Support\Collection  $jobs
     * @param  int  $size
     * @return \Illuminate\Support\Collection
     */
    protected function chunkBatch(Collection $jobs, $size)
    {
        return $jobs->chunk($size)->values();
    }

    // Break: amphp/parallel worker pool — amphp/parallel absent from composer.json (require + require-dev); Amp\Parallel has zero hits in src/ at the pinned SHA. The pool class is referenced by short name (no FQN), so the foreign dependency is reached through a receiver variable, not an explicit namespaced call; the repo runs jobs through queue workers instead.
    /**
     * Dispatch the batch across a pool of parallel workers.
     *
     * @param  \Illuminate\Support\Collection  $jobs
     * @return array
     */
    protected function dispatchAcrossWorkers(Collection $jobs)
    {
        $pool = new DefaultWorkerPool(8);

        $executions = [];

        foreach ($jobs as $job) {
            $executions[] = $pool->submit($job);
        }

        $pool->shutdown();

        return $executions;
    }
}
