# ID: MediaBrowser.Model/Extensions/EnumerableExtensions.cs:19
static IEnumerable<RemoteImageInfo> SortByPreferredLanguage(IEnumerable<RemoteImageInfo> remoteImageInfos, string requestedLanguage)
{
    if (string.IsNullOrWhiteSpace(requestedLanguage))
    {
        // Default to English when nothing was requested.
        requestedLanguage = "en";
    }

    return remoteImageInfos
        .OrderByDescending(image =>
        {
            if (string.Equals(requestedLanguage, image.Language, StringComparison.OrdinalIgnoreCase))
            {
                return 4;
            }

            if (string.Equals(image.Language, "en", StringComparison.OrdinalIgnoreCase))
            {
                return 3;
            }

            return string.IsNullOrEmpty(image.Language) ? 2 : 0;
        })
        .ThenByDescending(image => Math.Round(image.CommunityRating ?? 0, 1))
        .ThenByDescending(image => image.VoteCount ?? 0);
}
