# ID: Jellyfin.Api/Helpers/HlsCodecStringHelpers.cs:171
static string BuildH264CodecString(string? profile, int level)
{
    var codec = new StringBuilder("avc1", 11);

    if (string.Equals(profile, "high", StringComparison.OrdinalIgnoreCase))
    {
        codec.Append(".6400");
    }
    else if (string.Equals(profile, "main", StringComparison.OrdinalIgnoreCase))
    {
        codec.Append(".4D40");
    }
    else if (string.Equals(profile, "baseline", StringComparison.OrdinalIgnoreCase))
    {
        codec.Append(".42E0");
    }
    else
    {
        // Fall back to constrained baseline.
        codec.Append(".4240");
    }

    codec.Append(level.ToString("X2", CultureInfo.InvariantCulture));
    return codec.ToString();
}
