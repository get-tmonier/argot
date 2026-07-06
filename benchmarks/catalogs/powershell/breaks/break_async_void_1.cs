        // Break: fixture spliced at class-member level into InvokeRestMethodCommand.Common.cs.
        // Break: the hunk is a fire-and-forget async void member; the host's async surface is
        // Break: async Task consumed via await or GetAwaiter().GetResult().

        /// <summary>
        /// True when the response content type indicates a JSON payload.
        /// </summary>
        private static bool IsJsonContentType(HttpResponseMessage response)
        {
            string contentType = ContentHelper.GetContentType(response);
            return contentType != null && contentType.EndsWith("json", StringComparison.OrdinalIgnoreCase);
        }

        // Break: begin hunk — async void fire-and-forget; exceptions escape to the thread pool
        // Break: and the caller cannot await or observe completion.
        private int _cachedProbeStatus;

        private async void RefreshResponseProbeAsync(HttpClient client, Uri uri)
        {
            HttpResponseMessage probe = await client.GetAsync(uri, _cancelToken.Token);
            probe.EnsureSuccessStatusCode();
            _cachedProbeStatus = (int)probe.StatusCode;
        }
        // Break: end hunk

        /// <summary>
        /// Resets any cached probe state between pipeline invocations.
        /// </summary>
        private void ResetResponseProbe()
        {
            _cachedProbeStatus = 0;
        }

