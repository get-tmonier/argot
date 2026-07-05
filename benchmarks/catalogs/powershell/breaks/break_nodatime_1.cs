        // Break: fixture spliced at class-member level into engine/Utils.cs.
        // Break: decoy below mirrors the host's System.DateTime timestamping; the hunk does not.

        /// <summary>
        /// Returns the current UTC timestamp the way the engine already stamps events.
        /// </summary>
        internal static System.DateTime UtcTimestamp()
        {
            return System.DateTime.UtcNow;
        }

        // Break: begin hunk — NodaTime clock stamps the event instant; NodaTime is absent from the
        // Break: repo at the pinned SHA — time handling here uses System.DateTime / DateTimeOffset.
        using NodaTime;
        internal static Instant CurrentInstant()
        {
            return SystemClock.Instance.GetCurrentInstant();
        }
        // Break: end hunk

        /// <summary>
        /// True when the supplied interval has already elapsed.
        /// </summary>
        internal static bool HasElapsed(System.TimeSpan interval, System.DateTime since)
        {
            return System.DateTime.UtcNow - since > interval;
        }
