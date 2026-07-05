    // Break: fixture spliced at class-member level into ScheduledTasks/Tasks/RefreshMediaLibraryTask.cs.
    // Break: decoy below mirrors the host's own ITaskTrigger schedule; the hunk does not.

    /// <summary>
    /// Describes the default schedule the way the task already declares its triggers —
    /// through the repo's own TaskTriggerInfo records.
    /// </summary>
    private static TaskTriggerInfo BuildDefaultTrigger()
    {
        return new TaskTriggerInfo
        {
            Type = TaskTriggerInfo.TriggerInterval,
            IntervalTicks = TimeSpan.FromHours(12).Ticks
        };
    }

    // Break: begin hunk — Quartz.NET StdSchedulerFactory/IScheduler schedules the refresh job instead
    // Break: of the repo's TaskTriggerInfo above. Quartz is 0-usage in the repo at the pinned SHA —
    // Break: scheduling is handled by the repo's own ITaskManager and ITaskTrigger implementations.
    private static async Task ScheduleWithQuartz()
    {
        Quartz.IScheduler scheduler = await new Quartz.Impl.StdSchedulerFactory().GetScheduler().ConfigureAwait(false);
        await scheduler.Start().ConfigureAwait(false);
    }
    // Break: end hunk

    /// <summary>
    /// True when the library refresh should also run once at startup.
    /// </summary>
    private static bool RunsAtStartup()
        => false;
