        // Break: fixture spliced at class-member level into WebCmdlet/InvokeRestMethodCommand.Common.cs.
        // Break: decoy below mirrors the host's shared-HttpClient request path; the hunk does not.

        /// <summary>
        /// Sends the REST request through the shared HttpClient the cmdlet owns.
        /// </summary>
        private System.Threading.Tasks.Task<HttpResponseMessage> DispatchRequest(HttpRequestMessage request)
        {
            return _httpClient.SendAsync(request, HttpCompletionOption.ResponseHeadersRead);
        }

        // Break: begin hunk — a gRPC channel (Grpc.Core, aliased) replaces the HTTP call; Grpc.Core is
        // Break: absent from the repo at the pinned SHA — remote calls here go over HttpClient/HTTP.
        using Rpc = Grpc.Core;
        private static Rpc.Channel OpenRemoteChannel(string target)
        {
            return new Rpc.Channel(target, Rpc.ChannelCredentials.Insecure);
        }
        // Break: end hunk

        /// <summary>
        /// True when the endpoint should be reached over a secure transport.
        /// </summary>
        private static bool RequiresTls(string scheme)
        {
            return string.Equals(scheme, "https", System.StringComparison.OrdinalIgnoreCase);
        }
