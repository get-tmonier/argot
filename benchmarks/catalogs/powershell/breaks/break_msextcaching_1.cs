        // Break: fixture spliced at class-member level into engine/Modules/AnalysisCache.cs.
        // Break: decoy below mirrors the host's in-memory ConcurrentDictionary cache; the hunk does not.

        /// <summary>
        /// Serves an analysis entry from the file's own in-memory dictionary cache.
        /// </summary>
        private ModuleAnalysis LookupInMemory(string moduleName)
        {
            return _entries.TryGetValue(moduleName, out var analysis) ? analysis : null;
        }

        // Break: begin hunk — Microsoft.Extensions.Caching.Memory backs the analysis cache; that package
        // Break: is absent from the repo at the pinned SHA — this cache is a ConcurrentDictionary. HARD:
        // Break: reached fully qualified (no `using`) and its namespace root is Microsoft (repo-attested),
        // Break: so call_receiver is blind and no foreign import token appears. Genuine miss.
        private readonly Microsoft.Extensions.Caching.Memory.MemoryCache _memoryCache =
            new Microsoft.Extensions.Caching.Memory.MemoryCache(new Microsoft.Extensions.Caching.Memory.MemoryCacheOptions());

        private object ReadCached(string key)
        {
            if (_memoryCache.TryGetValue(key, out object value))
            {
                return value;
            }

            return null;
        }
        // Break: end hunk

        /// <summary>
        /// True when the cache has grown past its retention bound.
        /// </summary>
        private bool OverRetention(int entryCount)
        {
            return entryCount > MaxInMemoryEntries;
        }
