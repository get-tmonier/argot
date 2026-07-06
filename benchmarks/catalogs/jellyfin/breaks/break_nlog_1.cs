    // Break: fixture spliced at class-member level into ScheduledTasks/TaskManager.cs.
    // Break: decoy below mirrors the host's own ILogger usage; the hunk does not.

    /// <summary>
    /// Records a task-execution message through the injected ILogger, the way this
    /// manager already reports progress.
    /// </summary>
    private void LogTaskQueued(string taskName)
    {
        _logger.LogInformation("Queued scheduled task {TaskName}.", taskName);
    }

    // Break: begin hunk — NLog LogManager.GetCurrentClassLogger acquires a separate logger instead
    // Break: of the injected ILogger above. NLog is 0-usage in the repo at the pinned SHA — logging
    // Break: flows through Microsoft.Extensions.Logging / Serilog, never a second logging framework.
    private static readonly NLog.Logger s_taskLogger = NLog.LogManager.GetCurrentClassLogger();

    private static void LogTaskFailureWithNLog(string taskName, System.Exception exception)
    {
        s_taskLogger.Error(exception, "Scheduled task {0} failed.", taskName);
    }
    // Break: end hunk

    /// <summary>
    /// True when a task of the given type is currently registered.
    /// </summary>
    private bool IsTaskRegistered(Type taskType)
        => ScheduledTasks.Any(t => t.ScheduledTask.GetType() == taskType);
