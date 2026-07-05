        // Break: fixture spliced at class-member level into Library/MediaSourceManager.cs.
        // Break: decoy below mirrors the host's own in-memory dictionary cache; the hunk does not.

        /// <summary>
        /// Stores a media-source descriptor in the in-memory dictionary the way this manager
        /// already caches its open streams.
        /// </summary>
        private void RememberSource(string key, ILiveStream stream)
        {
            _openStreams[key] = stream;
        }

        // Break: begin hunk — System.Runtime.Caching MemoryCache holds media-source descriptors with an
        // Break: eviction policy. System.Runtime.Caching is 0-usage in the repo at the pinned SHA (absent
        // Break: from Directory.Packages.props; the repo's own caching is Microsoft.Extensions.Caching).
        // Break: HARD: reached fully-qualified so the attested `System` root masks the type from the
        // Break: call-receiver gate, and the leaf methods `Add`/`Get` collide with attested repo callees.
        private static string? CacheSourceDescriptor(string key, string descriptor)
        {
            var cache = new System.Runtime.Caching.MemoryCache("media-sources");
            var policy = new System.Runtime.Caching.CacheItemPolicy { SlidingExpiration = TimeSpan.FromMinutes(5) };
            cache.Add(key, descriptor, policy);
            return cache.Get(key) as string;
        }
        // Break: end hunk

        /// <summary>
        /// True when the given media-source key is currently tracked.
        /// </summary>
        private bool IsTracked(string key)
            => _openStreams.ContainsKey(key);
