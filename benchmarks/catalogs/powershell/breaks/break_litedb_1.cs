        // Break: fixture spliced at class-member level into engine/Modules/AnalysisCache.cs.
        // Break: decoy below mirrors the host's in-memory ConcurrentDictionary cache; the hunk does not.

        /// <summary>
        /// Reads a cached analysis entry the way this cache already serves lookups — from memory.
        /// </summary>
        private ModuleAnalysis ReadFromMemory(string moduleName)
        {
            return _entries.TryGetValue(moduleName, out var analysis) ? analysis : null;
        }

        // Break: begin hunk — LiteDB embedded document store persists the analysis cache; LiteDB is
        // Break: absent from the repo at the pinned SHA — this cache lives in a ConcurrentDictionary.
        using LiteDB;
        private static void PersistAnalysis(string path, ModuleAnalysis analysis)
        {
            using var db = new LiteDatabase(path);
            var col = db.GetCollection<ModuleAnalysis>("analysis");
            col.Upsert(analysis);
        }
        // Break: end hunk

        /// <summary>
        /// True when the cache has grown past its in-memory retention bound.
        /// </summary>
        private bool ExceedsRetention(int entryCount)
        {
            return entryCount > MaxInMemoryEntries;
        }
