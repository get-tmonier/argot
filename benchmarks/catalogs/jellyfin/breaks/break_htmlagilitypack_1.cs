        // Break: fixture spliced at class-member level into Plugins/Omdb/OmdbProvider.cs.
        // Break: decoy below mirrors the host's own System.Text.Json parse; the hunk does not.

        /// <summary>
        /// Reads the rating out of the OMDb JSON payload, the way this provider already
        /// deserialises its remote responses.
        /// </summary>
        private static string? ReadRating(JsonElement root)
        {
            return root.TryGetProperty("imdbRating", out var value) ? value.GetString() : null;
        }

        // Break: begin hunk — HtmlAgilityPack HtmlDocument scrapes a rating out of an HTML page
        // Break: instead of the JSON parse above. HtmlAgilityPack is 0-usage in the repo at the pinned
        // Break: SHA — providers consume structured JSON via System.Text.Json, never scraped HTML.
        using HtmlAgilityPack;
        private static string? ScrapeRatingFromHtml(string html)
        {
            var document = new HtmlDocument();
            document.LoadHtml(html);
            return document.DocumentNode.SelectSingleNode("//span[@class='rating']")?.InnerText;
        }
        // Break: end hunk

        /// <summary>
        /// True when the given imdb id is well-formed enough to query.
        /// </summary>
        private static bool IsValidImdbId(string imdbId)
            => imdbId.StartsWith("tt", StringComparison.Ordinal);
