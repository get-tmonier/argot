        // Break: fixture spliced at class-member level into remoting/common/throttlemanager.cs.
        // Break: the hunk queues operations through System.Threading.Channels; the host
        // Break: coordinates through its own operation queue, locks and event callbacks.

        /// <summary>
        /// Upper bound on operations admitted in a single submission burst.
        /// </summary>
        private const int MaxSubmitBurst = 32;

        /// <summary>
        /// True when the throttle limit still allows another operation to start.
        /// </summary>
        private bool CanStartOperation(int startedCount)
        {
            return startedCount < ThrottleLimit;
        }

        // Break: begin hunk — Channel<T> producer/consumer pump with async reader loop.
        private readonly System.Threading.Channels.Channel<IThrottleOperation> _operationChannel =
            System.Threading.Channels.Channel.CreateUnbounded<IThrottleOperation>();

        private async System.Threading.Tasks.Task PumpOperationsAsync()
        {
            while (await _operationChannel.Reader.WaitToReadAsync())
            {
                while (_operationChannel.Reader.TryRead(out IThrottleOperation operation))
                {
                    operation.StartOperation();
                }
            }
        }
        // Break: end hunk

        /// <summary>
        /// Removes every queued operation that has not yet been started.
        /// </summary>
        private void ClearQueuedOperations()
        {
            lock (_syncObject)
            {
                _operationsQueue.Clear();
            }
        }

