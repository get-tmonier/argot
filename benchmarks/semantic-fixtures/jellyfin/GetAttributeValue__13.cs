# ID: Emby.Server.Implementations/Library/PathExtensions.cs:20
static string? ReadAttributeValue(ReadOnlySpan<char> str, ReadOnlySpan<char> attribute)
{
    if (str.Length == 0)
    {
        throw new ArgumentException("String can't be empty.", nameof(str));
    }
    if (attribute.Length == 0)
    {
        throw new ArgumentException("String can't be empty.", nameof(attribute));
    }
    var limit = str.Length - attribute.Length - 2;
    var found = str.IndexOf(attribute, StringComparison.OrdinalIgnoreCase);
    while (found > -1 && found < limit)
    {
        var after = found + attribute.Length;
        if (found > 0)
        {
            var closer = str[found - 1] switch { '[' => ']', '(' => ')', '{' => '}', _ => '\0' };
            if (closer != '\0' && (str[after] == '=' || str[after] == '-'))
            {
                var closeAt = str[after..].IndexOf(closer);
                if (closeAt > 1)
                {
                    return str[(after + 1)..(after + closeAt)].Trim().ToString();
                }
            }
        }
        str = str[after..];
        found = str.IndexOf(attribute, StringComparison.OrdinalIgnoreCase);
    }
    if (attribute.Equals("imdbid", StringComparison.OrdinalIgnoreCase))
    {
        return ProviderIdParsers.TryFindImdbId(str, out var imdbId) ? imdbId.ToString() : null;
    }
    if (attribute.Equals("tmdbid", StringComparison.OrdinalIgnoreCase))
    {
        return ReadAttributeValue(str, "tmdb");
    }
    return null;
}
