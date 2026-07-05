        // Break: fixture spliced at class-member level into remoting/common/throttlemanager.cs.
        // Break: decoy below mirrors the host's own operation-queue coordination; the hunk does not.

        /// <summary>
        /// Starts the next queued operation under the throttle manager's own coordination.
        /// </summary>
        private void StartNextOperation()
        {
            lock (_syncObject)
            {
                if (_operationQueue.Count > 0)
                {
                    _operationQueue.Dequeue().Start();
                }
            }
        }

        // Break: begin hunk — Quartz.NET scheduler drives throttled operations; Quartz is absent from
        // Break: the repo at the pinned SHA — throttling is coordinated by the manager's own queue.
        private static async System.Threading.Tasks.Task ScheduleThrottledSweep()
        {
            var scheduler = await new Quartz.Impl.StdSchedulerFactory().GetScheduler();
            await scheduler.Start();
        }
        // Break: end hunk

        /// <summary>
        /// True when the throttle limit has been reached.
        /// </summary>
        private bool AtThrottleLimit(int active)
        {
            return active >= _throttleLimit;
        }
