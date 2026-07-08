# ID: Emby.Naming/Audio/AlbumParser.cs:34
static bool SpansMultipleDiscs(NamingOptions options, string path)
{
    var baseName = Path.GetFileName(path);
    if (baseName.Length == 0)
    {
        return false;
    }

    // Collapse punctuation and whitespace down to single spaces before matching.
    baseName = CleanRegex().Replace(baseName, " ");
    ReadOnlySpan<char> normalized = baseName.AsSpan().TrimStart();

    foreach (var stackingPrefix in options.AlbumStackingPrefixes)
    {
        if (!normalized.StartsWith(stackingPrefix, StringComparison.OrdinalIgnoreCase))
        {
            continue;
        }

        var afterPrefix = normalized.Slice(stackingPrefix.Length).Trim();
        if (int.TryParse(afterPrefix.LeftPart(' '), CultureInfo.InvariantCulture, out _))
        {
            return true;
        }
    }

    return false;
}
