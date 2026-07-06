        // Break: fixture spliced at class-member level into Manager/ProviderManager.cs.
        // Break: decoy below mirrors the host's own HttpClient remote fetch; the hunk does not.

        /// <summary>
        /// Fetches a remote image listing through the shared HttpClient, the way this manager
        /// already reaches its remote image providers.
        /// </summary>
        private async Task<string> FetchRemoteImages(string url, CancellationToken cancellationToken)
        {
            using var client = _httpClientFactory.CreateClient(NamedClient.Default);
            return await client.GetStringAsync(url, cancellationToken).ConfigureAwait(false);
        }

        // Break: begin hunk — Refit RestService.For builds a typed HTTP client for the remote image
        // Break: API. Refit is 0-usage in the repo at the pinned SHA (no `using Refit;` here — the tell
        // Break: is the bare RestService.For callee) — providers issue requests through raw HttpClient.
        private interface IRemoteImageApi
        {
            Task<string> GetImagesAsync(string itemId);
        }

        private static Task<string> FetchViaRefit(string baseUrl, string itemId)
        {
            IRemoteImageApi api = RestService.For<IRemoteImageApi>(baseUrl);
            return api.GetImagesAsync(itemId);
        }
        // Break: end hunk

        /// <summary>
        /// True when at least one remote image provider is enabled for the item.
        /// </summary>
        private bool HasRemoteProviders(BaseItem item)
            => GetRemoteImageProviders(item, false).Any();
