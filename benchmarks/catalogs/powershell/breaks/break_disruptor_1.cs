        // Break: fixture spliced at class-member level into engine/hostifaces/RunspacePool.cs.
        // Break: decoy below mirrors the host's own lock-guarded forwarded-event bookkeeping; the hunk does not.

        /// <summary>
        /// True when a listener is registered for forwarded events, checked under the pool's own lock.
        /// </summary>
        private bool HasForwardedEventListener()
        {
            lock (_syncObject)
            {
                return InternalForwardEvent != null;
            }
        }

        // Break: begin hunk — LMAX Disruptor ring buffer pumps queued runspace work through a receiver
        // Break: variable; Disruptor-net is absent from the repo at the pinned SHA (0 grep hits for
        // Break: `Disruptor` in src/*.cs and 0 PackageReference entries in *.csproj repo-wide) — queued
        // Break: work here is coordinated through _syncObject locks, not a lock-free ring buffer. MEDIUM:
        // Break: reached through a receiver variable (disruptor.Start()).
        private void PumpRunspaceWorkViaDisruptor(int ringSize)
        {
            var disruptor = new Disruptor.Dsl.Disruptor<RunspaceWorkEvent>(() => new RunspaceWorkEvent(), ringSize);
            disruptor.HandleEventsWith(new RunspaceWorkHandler());
            disruptor.Start();
        }
        // Break: end hunk

        /// <summary>
        /// Clears the internal forward-event handler under the pool's own lock.
        /// </summary>
        private void ClearForwardedEventListener()
        {
            lock (_syncObject)
            {
                InternalForwardEvent = null;
            }
        }
