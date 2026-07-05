        // Break: fixture spliced at class-member level into WebCmdlet/InvokeRestMethodCommand.Common.cs.
        // Break: decoy below mirrors the host's shared-HttpClient request path; the hunk does not.

        /// <summary>
        /// Issues the REST request through the shared HttpClient the cmdlet already owns.
        /// </summary>
        private System.Threading.Tasks.Task<HttpResponseMessage> SendThroughSharedClient(HttpRequestMessage request)
        {
            return _httpClient.SendAsync(request, HttpCompletionOption.ResponseHeadersRead);
        }

        // Break: begin hunk — Flurl.Http fluent client fetches the REST payload; Flurl is absent from
        // Break: the repo at the pinned SHA — every request goes through HttpClient/HttpRequestMessage.
        using Flurl.Http;
        private static async System.Threading.Tasks.Task<string> FetchWithFlurl(string endpoint)
        {
            return await endpoint.GetStringAsync();
        }
        // Break: end hunk

        /// <summary>
        /// True when the response content type indicates a JSON body.
        /// </summary>
        private static bool IsJsonResponse(string contentType)
        {
            return contentType != null && contentType.Contains("json");
        }
