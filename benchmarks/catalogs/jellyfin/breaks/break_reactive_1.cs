        // Break: fixture spliced at class-member level into Session/SessionManager.cs.
        // Break: decoy below mirrors the host's own event-based notification; the hunk does not.

        /// <summary>
        /// Raises the session-activity event the way SessionManager already notifies
        /// listeners of state changes.
        /// </summary>
        private void NotifyActivity(SessionInfo session)
        {
            SessionActivity?.Invoke(this, new SessionEventArgs(session));
        }

        // Break: begin hunk — System.Reactive (Rx.NET) Observable/Subject drives a reactive heartbeat
        // Break: stream instead of the plain event above. System.Reactive is 0-usage in the repo at the
        // Break: pinned SHA — session signalling uses C# events and Timer, never a reactive library.
        using System.Reactive.Linq;
        using System.Reactive.Subjects;
        private readonly Subject<SessionInfo> _sessionPings = new Subject<SessionInfo>();

        private System.IDisposable StartHeartbeat()
        {
            return Observable
                .Interval(System.TimeSpan.FromSeconds(30))
                .Subscribe(_ => _sessionPings.OnNext(new SessionInfo(null, string.Empty)));
        }
        // Break: end hunk

        /// <summary>
        /// True when the session currently holds an active live-stream lease.
        /// </summary>
        private bool HasLiveStream(string sessionId)
            => _activeLiveStreamSessions.ContainsKey(sessionId);
