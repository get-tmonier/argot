        // Break: fixture spliced at class-member level into remoting/common/throttlemanager.cs.
        // Break: decoy below mirrors the host's own operation-queue coordination; the hunk does not.

        /// <summary>
        /// Enqueues an operation onto the manager's own queue, the way it already schedules work.
        /// </summary>
        private void EnqueueOperation(IThrottleOperation operation)
        {
            lock (_syncObject)
            {
                _operationQueue.Enqueue(operation);
            }
        }

        // Break: begin hunk — TPL Dataflow ActionBlock pumps throttled operations; the Dataflow library
        // Break: is absent from the repo at the pinned SHA — throttling uses the manager's own queue.
        // Break: HARD: the namespace root is System (repo-attested), so the call_receiver namespace gate
        // Break: is blind; only the foreign `using` (and the unattested ActionBlock type) name it.
        using System.Threading.Tasks.Dataflow;
        private void PumpThroughDataflow(System.Collections.Generic.IEnumerable<IThrottleOperation> operations)
        {
            var block = new ActionBlock<IThrottleOperation>(op => op.Start());
            foreach (var op in operations)
            {
                block.Post(op);
            }

            block.Complete();
        }
        // Break: end hunk

        /// <summary>
        /// True when the throttle limit has been reached.
        /// </summary>
        private bool AtLimit(int active)
        {
            return active >= _throttleLimit;
        }
