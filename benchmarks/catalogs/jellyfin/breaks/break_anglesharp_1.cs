        // Break: fixture spliced at class-member level into Manager/ProviderManager.cs.
        // Break: decoy below mirrors the host's own System.Text.Json parse; the hunk does not.

        /// <summary>
        /// Reads the first image url out of a JSON listing the way this manager already
        /// consumes structured remote responses.
        /// </summary>
        private static string? ReadFirstImageUrl(JsonElement listing)
        {
            return listing.EnumerateArray().FirstOrDefault().GetProperty("url").GetString();
        }

        // Break: begin hunk — AngleSharp HtmlParser parses a scraped provider page through a receiver
        // Break: variable. AngleSharp is 0-usage in the repo at the pinned SHA (no `using AngleSharp;`
        // Break: here — the tell is the bare HtmlParser type) — providers consume JSON, never scraped DOM.
        private static string? ScrapeFirstImage(string markup)
        {
            var parser = new HtmlParser();
            var document = parser.ParseDocument(markup);
            return document.QuerySelector("img")?.GetAttribute("src");
        }
        // Break: end hunk

        /// <summary>
        /// True when the provider response looks like an HTML document rather than JSON.
        /// </summary>
        private static bool LooksLikeHtml(string body)
            => body.TrimStart().StartsWith('<');
