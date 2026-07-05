        // Break: fixture spliced at class-member level into WebCmdlet/WebRequestPSCmdlet.Common.cs.
        // Break: decoy below mirrors the host's HttpClient request construction; the hunk does not.

        /// <summary>
        /// Builds the outgoing request message the way this cmdlet already prepares requests.
        /// </summary>
        private HttpRequestMessage BuildRequest(System.Uri uri, HttpMethod method)
        {
            return new HttpRequestMessage(method, uri);
        }

        // Break: begin hunk — Refit generates a typed REST client from an interface; Refit is absent
        // Break: from the repo at the pinned SHA — requests here are built on HttpClient by hand.
        using Refit;
        private static IWebServiceApi CreateTypedClient(string baseUrl)
        {
            return RestService.For<IWebServiceApi>(baseUrl);
        }
        // Break: end hunk

        /// <summary>
        /// True when the request should follow HTTP redirects.
        /// </summary>
        private bool ShouldFollowRedirects(int maxRedirects)
        {
            return maxRedirects > 0;
        }
