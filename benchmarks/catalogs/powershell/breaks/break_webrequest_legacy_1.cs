        // Break: fixture spliced at class-member level into WebRequestPSCmdlet.Common.cs.
        // Break: the hunk uses the deprecated WebRequest.Create/HttpWebRequest surface of
        // Break: System.Net; this file standardizes on HttpClient/HttpRequestMessage.

        /// <summary>
        /// The header carrying the redirect target on a 3xx response.
        /// </summary>
        private const string LocationHeaderName = "Location";

        /// <summary>
        /// True when the status code denotes any redirection class response.
        /// </summary>
        private static bool IsRedirectStatus(HttpStatusCode statusCode)
        {
            int code = (int)statusCode;
            return code >= 300 && code < 400;
        }

        // Break: begin hunk — legacy synchronous WebRequest.Create/GetResponse pipeline.
        private static string FetchRedirectTarget(Uri uri)
        {
            System.Net.HttpWebRequest request = (System.Net.HttpWebRequest)System.Net.WebRequest.Create(uri);
            request.Method = "HEAD";
            request.AllowAutoRedirect = false;

            using (System.Net.WebResponse response = request.GetResponse())
            {
                return response.Headers[LocationHeaderName];
            }
        }
        // Break: end hunk

        /// <summary>
        /// Strips any user information from a redirect target before it is followed.
        /// </summary>
        private static Uri SanitizeRedirectUri(Uri uri)
        {
            var builder = new UriBuilder(uri) { UserName = string.Empty, Password = string.Empty };
            return builder.Uri;
        }

