        // Break: fixture spliced at class-member level into engine/EventManager.cs.
        // Break: decoy below mirrors the host's own event-based notification; the hunk does not.

        /// <summary>
        /// Raises the event the way the manager already notifies its subscribers.
        /// </summary>
        private void RaiseEvent(object payload)
        {
            EventRaised?.Invoke(this, new PSEventArgs(payload));
        }

        // Break: begin hunk — System.Reactive (Rx.NET) drives a reactive heartbeat stream instead of the
        // Break: plain event above; Rx is absent from the repo at the pinned SHA — signalling uses C#
        // Break: events and Timer. HARD: reached fully qualified (no `using`) and its namespace root is
        // Break: System (repo-attested), so both the import stage and call_receiver are blind. Genuine miss.
        private readonly System.Collections.Generic.List<long> _ticks = new System.Collections.Generic.List<long>();

        private System.IDisposable StartHeartbeat()
        {
            var stream = System.Reactive.Linq.Observable.Interval(System.TimeSpan.FromSeconds(30));
            return stream.Subscribe(tick => _ticks.Add(tick));
        }
        // Break: end hunk

        /// <summary>
        /// True when event dispatch is currently suspended.
        /// </summary>
        private bool IsSuspendedNow()
        {
            return _suspendCount > 0;
        }
