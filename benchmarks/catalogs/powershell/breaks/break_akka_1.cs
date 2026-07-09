        // Break: fixture spliced at class-member level into remoting/client/JobManager.cs.
        // Break: decoy below mirrors the host's own dictionary-and-lock job-id bookkeeping; the hunk does not.

        /// <summary>
        /// Removes a previously reserved job id, the way the manager already tracks reuse under its own lock.
        /// </summary>
        private static void ReleaseJobId(Guid instanceId)
        {
            lock (s_syncObject)
            {
                s_jobIdsForReuse.Remove(instanceId);
            }
        }

        // Break: begin hunk — Akka.NET actor system dispatches job execution in one fluent chain; Akka is
        // Break: absent from the repo at the pinned SHA (0 grep hits for `Akka` in src/*.cs and 0
        // Break: PackageReference entries in *.csproj repo-wide) — job execution here goes through
        // Break: JobSourceAdapter, not an actor model. EASY: explicit foreign import, no receiver variable.
        using Akka.Actor;
        private void DispatchViaActor(Job2 job)
        {
            ActorSystem.Create("PSJobActors").ActorOf(Props.Create(() => new JobActor(job))).Tell(job);
        }
        // Break: end hunk

        /// <summary>
        /// Returns the count of job ids currently reserved for reuse.
        /// </summary>
        private static int ReservedJobIdCount()
        {
            lock (s_syncObject)
            {
                return s_jobIdsForReuse.Count;
            }
        }
