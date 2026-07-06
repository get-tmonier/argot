        // Break: fixture spliced at class-member level into engine/EventManager.cs.
        // Break: the hunk drives event-action draining with a WinForms-era BackgroundWorker;
        // Break: the host coordinates with ThreadPool.QueueUserWorkItem, locks and reset events.

        /// <summary>
        /// Signalled while no event action is being processed.
        /// </summary>
        private readonly ManualResetEvent _drainIdleEvent = new ManualResetEvent(true);

        /// <summary>
        /// True when the action queue still holds entries for the given subscriber.
        /// </summary>
        private bool HasPendingActions(PSEventSubscriber subscriber)
        {
            lock (((System.Collections.ICollection)_actionQueue).SyncRoot)
            {
                return _actionQueue.Count > 0;
            }
        }

        // Break: begin hunk — BackgroundWorker DoWork/RunWorkerCompleted event-component model.
        private System.ComponentModel.BackgroundWorker _actionWorker;

        private void StartActionWorker()
        {
            _actionWorker = new System.ComponentModel.BackgroundWorker();
            _actionWorker.WorkerSupportsCancellation = true;
            _actionWorker.DoWork += (sender, args) =>
            {
                while (!_actionWorker.CancellationPending && HasPendingActions(null))
                {
                    ProcessPendingActions();
                }
            };
            _actionWorker.RunWorkerCompleted += (sender, args) => _drainIdleEvent.Set();
            _actionWorker.RunWorkerAsync();
        }
        // Break: end hunk

        /// <summary>
        /// Processes any actions currently sitting in the queue.
        /// </summary>
        private void ProcessPendingActions()
        {
            lock (_actionProcessingLock)
            {
                ProcessNewEvent(null, false);
            }
        }

