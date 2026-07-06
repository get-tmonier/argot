    // Break: fixture spliced at class-member level into ScheduledTasks/ScheduledTaskWorker.cs.
    // Break: decoy below mirrors the host's own in-process task execution; the hunk does not.

    /// <summary>
    /// Runs the wrapped scheduled task in-process, the way this worker already executes
    /// its queued work.
    /// </summary>
    private Task RunInProcess(CancellationToken cancellationToken)
    {
        return ScheduledTask.ExecuteAsync(new Progress<double>(), cancellationToken);
    }

    // Break: begin hunk — Hangfire BackgroundJobClient hands the task off to a Hangfire server
    // Break: instead of the in-process runner above. Hangfire is 0-usage in the repo at the pinned
    // Break: SHA — background work runs through the repo's own ITaskManager / IScheduledTaskWorker.
    using Hangfire;
    private static readonly Hangfire.BackgroundJobClient s_backgroundJobs = new Hangfire.BackgroundJobClient();

    private void EnqueueWithHangfire()
    {
        s_backgroundJobs.Enqueue(() => ScheduledTask.ExecuteAsync(new Progress<double>(), CancellationToken.None));
    }
    // Break: end hunk

    /// <summary>
    /// True when this worker currently has a triggered execution running.
    /// </summary>
    private bool IsExecuting()
        => State == TaskState.Running;
