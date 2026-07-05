    // Break: fixture spliced at class-member level into Controllers/PlaystateController.cs.
    // Break: decoy below mirrors the host's own DateTime.UtcNow stamping; the hunk does not.

    /// <summary>
    /// Stamps a playback event with the current UTC time the way this controller already
    /// records progress — through the framework's own DateTime.
    /// </summary>
    private static DateTime StampNow()
    {
        return DateTime.UtcNow;
    }

    // Break: begin hunk — NodaTime SystemClock (reached through an aliased using of the NodaTime
    // Break: namespace) stamps the event instead of DateTime above. NodaTime is 0-usage in the repo
    // Break: at the pinned SHA — time handling uses System DateTime/DateTimeOffset, never NodaTime.
    using Clock = NodaTime;
    private static long StampPlaybackInstant()
    {
        Clock.Instant now = Clock.SystemClock.Instance.GetCurrentInstant();
        return now.ToUnixTimeMilliseconds();
    }
    // Break: end hunk

    /// <summary>
    /// True when the supplied play session id is present and non-empty.
    /// </summary>
    private static bool HasPlaySession(string? playSessionId)
        => !string.IsNullOrEmpty(playSessionId);
