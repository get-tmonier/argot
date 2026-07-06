        // Break: fixture spliced at class-member level into management/Process.cs.
        // Break: decoy below mirrors the host's synchronous in-process work; the hunk does not.

        /// <summary>
        /// Runs the process-stop work inline, the way this cmdlet already performs its actions.
        /// </summary>
        private void StopInline(System.Diagnostics.Process process)
        {
            process.Kill();
        }

        // Break: begin hunk — Hangfire hands background process work to a Hangfire server; Hangfire is
        // Break: absent from the repo at the pinned SHA — work runs inline on the cmdlet thread. HARD:
        // Break: the leaf method Enqueue collides with attested repo callees, so the fixture reaches
        // Break: Hangfire through its fully-qualified type (root Hangfire is foreign), not the leaf.
        private static readonly Hangfire.BackgroundJobClient s_jobs = new Hangfire.BackgroundJobClient();

        private static void QueueProcessCleanup(int processId)
        {
            s_jobs.Enqueue(() => System.Diagnostics.Process.GetProcessById(processId).Kill());
        }
        // Break: end hunk

        /// <summary>
        /// True if the supplied process name contains wildcard characters.
        /// </summary>
        private static bool HasWildcard(string processName)
        {
            return WildcardPattern.ContainsWildcardCharacters(processName);
        }
