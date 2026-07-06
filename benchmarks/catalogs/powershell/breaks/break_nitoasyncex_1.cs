        // Break: fixture spliced at class-member level into engine/EventManager.cs.
        // Break: decoy below mirrors the host's lock-based event coordination; the hunk does not.

        /// <summary>
        /// Drains the pending event queue under the manager's own lock, the way it already does.
        /// </summary>
        private void DrainPendingEvents()
        {
            lock (_syncRoot)
            {
                while (_pending.Count > 0)
                {
                    Dispatch(_pending.Dequeue());
                }
            }
        }

        // Break: begin hunk — Nito.AsyncEx AsyncLock serializes async event dispatch; Nito.AsyncEx is
        // Break: absent from the repo at the pinned SHA — coordination uses lock / ThreadPool / events.
        using Nito.AsyncEx;
        private readonly AsyncLock _dispatchGate = new AsyncLock();

        private async System.Threading.Tasks.Task DispatchGuardedAsync(object payload)
        {
            using (await _dispatchGate.LockAsync())
            {
                Dispatch(payload);
            }
        }
        // Break: end hunk

        /// <summary>
        /// True when event dispatch is currently suspended.
        /// </summary>
        private bool IsSuspended()
        {
            return _suspendCount > 0;
        }
