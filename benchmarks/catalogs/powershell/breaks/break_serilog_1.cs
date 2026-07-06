        // Break: fixture spliced at class-member level into utils/Telemetry.cs.
        // Break: the hunk logs through Serilog, a package absent from the repo at the pinned SHA;
        // Break: the host's telemetry goes through the Microsoft.ApplicationInsights client.

        /// <summary>
        /// The event name reported when a telemetry payload is dropped locally.
        /// </summary>
        private const string DroppedTelemetryEventName = "TelemetryDropped";

        /// <summary>
        /// True when the environment opt-out disables all telemetry emission.
        /// </summary>
        private static bool IsTelemetryOptedOut()
        {
            return !CanSendTelemetry;
        }

        // Break: begin hunk — Serilog file logger for telemetry events (foreign package).
        private static readonly Serilog.ILogger s_fileLogger = new Serilog.LoggerConfiguration()
            .MinimumLevel.Information()
            .WriteTo.File("powershell-telemetry.log")
            .CreateLogger();

        private static void LogTelemetryEvent(string eventName, double value)
        {
            s_fileLogger.Information("telemetry event {EventName} value {Value}", eventName, value);
        }
        // Break: end hunk

        /// <summary>
        /// Names the metric dimension used for the anonymized session identifier.
        /// </summary>
        private const string SessionIdDimension = "SessionId";

