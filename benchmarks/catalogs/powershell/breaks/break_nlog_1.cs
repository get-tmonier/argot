        // Break: fixture spliced at class-member level into Process.cs (GetProcessCommand Privates region).
        // Break: decoy below mirrors the host's WriteVerbose diagnostic channel; the hunk does not.

        /// <summary>
        /// Writes a verbose diagnostic line describing a process lookup outcome.
        /// </summary>
        private void WriteProcessLookupVerbose(string processName, int matchCount)
        {
            WriteVerbose(StringUtil.Format("{0}: {1} match(es)", processName, matchCount));
        }

        // Break: begin hunk — NLog file logger records process lookups; NLog is absent from the
        // Break: repo at the pinned SHA — diagnostics flow through WriteVerbose/WriteDebug instead.
        private static readonly NLog.Logger s_processLogger = NLog.LogManager.GetCurrentClassLogger();

        private void LogProcessLookup(string processName, int matchCount)
        {
            s_processLogger.Info("process lookup {0}: {1} match(es)", processName, matchCount);
        }
        // Break: end hunk

        /// <summary>
        /// True if the supplied process name contains wildcard characters.
        /// </summary>
        private static bool IsWildcardName(string processName)
        {
            return WildcardPattern.ContainsWildcardCharacters(processName);
        }

