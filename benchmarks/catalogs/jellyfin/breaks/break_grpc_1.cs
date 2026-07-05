        // Break: fixture spliced at class-member level into HdHomerun/HdHomerunHost.cs.
        // Break: decoy below mirrors the host's own HttpClient tuner call; the hunk does not.

        /// <summary>
        /// Fetches tuner status over HTTP through the shared HttpClient, the way this host
        /// already talks to its HDHomeRun devices.
        /// </summary>
        private async Task<string> GetTunerStatus(TunerHostInfo info, CancellationToken cancellationToken)
        {
            using var client = _httpClientFactory.CreateClient(NamedClient.Default);
            return await client.GetStringAsync(GetApiUrl(info) + "/status.json", cancellationToken).ConfigureAwait(false);
        }

        // Break: begin hunk — Grpc.Net.Client GrpcChannel opens a gRPC control channel to the tuner
        // Break: instead of the HttpClient above. Grpc.Net.Client is 0-usage in the repo at the pinned
        // Break: SHA — every device call goes through IHttpClientFactory / HttpClient over REST.
        using Grpc.Net.Client;
        private static async Task<string> ProbeTunerViaGrpc(string address)
        {
            using GrpcChannel channel = GrpcChannel.ForAddress(address);
            await channel.ConnectAsync().ConfigureAwait(false);
            return channel.State.ToString();
        }
        // Break: end hunk

        /// <summary>
        /// True when the tuner host descriptor points at a discoverable device url.
        /// </summary>
        private static bool HasDeviceUrl(TunerHostInfo info)
            => !string.IsNullOrWhiteSpace(info.Url);
