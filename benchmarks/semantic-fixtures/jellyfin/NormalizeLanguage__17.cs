# ID: MediaBrowser.Providers/Plugins/Tmdb/TmdbUtils.cs:140
static string? CanonicalizeLanguageCode(string? language, string? countryCode = null)
{
    if (string.IsNullOrEmpty(language))
    {
        return language;
    }

    // Latin American Spanish maps onto a concrete regional variant.
    if (string.Equals(language, "es-419", StringComparison.OrdinalIgnoreCase) && !string.IsNullOrEmpty(countryCode))
    {
        language = string.Equals(countryCode, "AR", StringComparison.OrdinalIgnoreCase) ? "es-AR" : "es-MX";
    }

    var segments = language.Split('-');
    if (segments.Length != 2)
    {
        return language;
    }

    // TMDb does not support Swiss locales, so fall back to the bare language.
    if (string.Equals(segments[1], "CH", StringComparison.OrdinalIgnoreCase))
    {
        return segments[0];
    }

    return segments[0] + "-" + segments[1].ToUpperInvariant();
}
