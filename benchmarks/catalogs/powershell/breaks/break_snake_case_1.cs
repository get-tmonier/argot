        // Break: fixture spliced at class-member level into engine/Modules/AnalysisCache.cs.
        // Break: the hunk uses snake_case methods/locals and a camelCase public property;
        // Break: the repo is PascalCase for members with _camelCase/s_camelCase fields.

        /// <summary>
        /// Tracks when the cache index was last compacted.
        /// </summary>
        private DateTime _lastCompactTime = DateTime.MinValue;

        /// <summary>
        /// True when the cache entry is newer than the module file it describes.
        /// </summary>
        private static bool IsEntryCurrent(ModuleCacheEntry entry, DateTime lastWriteTime)
        {
            return entry.ModulePathLastWriteTime >= lastWriteTime;
        }

        // Break: begin hunk — snake_case member morphology and camelCase public property.
        public int cacheEntryLimit { get; set; } = 100;

        private static string get_cache_store_path(string module_name)
        {
            string base_path = Path.GetTempPath();
            return Path.Combine(base_path, module_name);
        }

        private void purge_stale_entries(int max_age_days)
        {
            DateTime cut_off = DateTime.Now.AddDays(-max_age_days);
            foreach (string module_name in Entries.Keys)
            {
                if (File.GetLastWriteTime(get_cache_store_path(module_name)) < cut_off)
                {
                    Entries.TryRemove(module_name, out _);
                }
            }
        }
        // Break: end hunk

        /// <summary>
        /// Schedules a compaction if enough time has elapsed since the previous one.
        /// </summary>
        private void MaybeCompactEntries()
        {
            if ((DateTime.Now - _lastCompactTime) > TimeSpan.FromHours(1))
            {
                _lastCompactTime = DateTime.Now;
            }
        }

