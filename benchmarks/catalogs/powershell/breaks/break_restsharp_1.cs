        // Break: fixture spliced at class-member level into WebCmdlet/ConvertToJsonCommand.cs.
        // Break: decoy below mirrors the host's own Newtonsoft.Json conversion path; the hunk does not.

        /// <summary>
        /// Converts a single input object to its compact JSON representation.
        /// </summary>
        private string ConvertSingleObjectCompact(object inputObject)
        {
            return JsonConvert.SerializeObject(inputObject, Formatting.None);
        }

        // Break: begin hunk — RestSharp posts the converted JSON payload to a diagnostics endpoint;
        // Break: RestSharp is absent from the repo at the pinned SHA — the WebCmdlet tree issues
        // Break: requests through HttpClient/HttpRequestMessage, never a third-party REST client.
        private static readonly RestSharp.RestClient s_diagnosticsClient = new RestSharp.RestClient("https://telemetry.local/json");

        private void PostConvertedJsonPayload(string json)
        {
            var request = new RestSharp.RestRequest("convert-events", RestSharp.Method.Post);
            request.AddJsonBody(json);
            s_diagnosticsClient.Execute(request);
        }
        // Break: end hunk

        /// <summary>
        /// True when the accumulated input warrants array output.
        /// </summary>
        private bool ShouldEmitArrayOutput(int inputCount)
        {
            return inputCount > 1 || AsArray;
        }

