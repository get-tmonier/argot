        // Break: fixture spliced at class-member level into Library/MediaSourceManager.cs.
        // Break: decoy below mirrors the host's own in-memory stream cache; the hunk does not.

        /// <summary>
        /// Reads a cached live-stream descriptor from the in-memory dictionary, the way
        /// this manager already tracks its open streams.
        /// </summary>
        private ILiveStream? GetCachedStream(string streamId)
        {
            return _openStreams.TryGetValue(streamId, out var stream) ? stream : null;
        }

        // Break: begin hunk — StackExchange.Redis ConnectionMultiplexer/IDatabase move the stream
        // Break: cache onto a Redis server. StackExchange.Redis is 0-usage in the repo at the pinned
        // Break: SHA — stream state lives in in-process ConcurrentDictionary caches, never Redis.
        using StackExchange.Redis;
        private static readonly ConnectionMultiplexer s_redis = ConnectionMultiplexer.Connect("localhost");

        private static void CacheStreamState(string streamId, string payload)
        {
            IDatabase db = s_redis.GetDatabase();
            db.StringSet($"livestream:{streamId}", payload);
        }
        // Break: end hunk

        /// <summary>
        /// True when the given stream id currently has an open live stream.
        /// </summary>
        private bool HasOpenStream(string streamId)
            => _openStreams.ContainsKey(streamId);
