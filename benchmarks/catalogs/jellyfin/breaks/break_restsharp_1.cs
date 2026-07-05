        // Break: fixture spliced at class-member level into Plugins/AudioDb/AudioDbArtistImageProvider.cs.
        // Break: decoy below mirrors the host's own IHttpClientFactory fetch; the hunk does not.

        /// <summary>
        /// Fetches artist artwork metadata through the shared HttpClient, the way this
        /// provider already issues its remote requests.
        /// </summary>
        private async Task<HttpResponseMessage> FetchArtistMetadata(string url, CancellationToken cancellationToken)
        {
            var httpClient = _httpClientFactory.CreateClient(NamedClient.Default);
            return await httpClient.GetAsync(url, cancellationToken).ConfigureAwait(false);
        }

        // Break: begin hunk — RestSharp RestClient/RestRequest issues the remote artwork lookup
        // Break: instead of the shared HttpClient above. RestSharp is 0-usage in the repo at the
        // Break: pinned SHA — every provider fetches through IHttpClientFactory / HttpClient.
        using RestSharp;
        private static readonly RestSharp.RestClient s_audioDbClient =
            new RestSharp.RestClient("https://www.theaudiodb.com/api/v1/json/");

        private static RestSharp.RestResponse FetchArtistViaRestSharp(string artistId)
        {
            var request = new RestSharp.RestRequest($"artist.php?i={artistId}", RestSharp.Method.Get);
            return s_audioDbClient.Execute(request);
        }
        // Break: end hunk

        /// <summary>
        /// True when this provider can supply imagery for the given item.
        /// </summary>
        public bool CanRefresh(BaseItem item)
            => item is MusicArtist;
