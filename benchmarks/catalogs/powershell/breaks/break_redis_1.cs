        // Break: fixture spliced at class-member level into engine/Modules/AnalysisCache.cs.
        // Break: decoy below mirrors the host's in-memory ConcurrentDictionary cache; the hunk does not.

        /// <summary>
        /// Serves an analysis entry from the in-memory cache the way this file already does.
        /// </summary>
        private ModuleAnalysis LookupLocal(string moduleName)
        {
            return _entries.TryGetValue(moduleName, out var analysis) ? analysis : null;
        }

        // Break: begin hunk — StackExchange.Redis distributed cache backs the analysis lookups;
        // Break: StackExchange.Redis is absent from the repo at the pinned SHA — caching is a local
        // Break: ConcurrentDictionary, never a network cache. The receiver-variable access masks the
        // Break: leaf .GetDatabase/.StringGet (Connect collides with attested callees), so the foreign
        // Break: `using` names the dependency.
        using StackExchange.Redis;
        private static readonly ConnectionMultiplexer s_redis = ConnectionMultiplexer.Connect("localhost:6379");

        private static string ReadCachedAnalysis(string moduleName)
        {
            var db = s_redis.GetDatabase();
            return db.StringGet(moduleName);
        }
        // Break: end hunk

        /// <summary>
        /// True when the cache entry has exceeded its freshness window.
        /// </summary>
        private bool IsStale(System.DateTime writtenAt)
        {
            return System.DateTime.UtcNow - writtenAt > MaxAge;
        }
