# ID: MediaBrowser.MediaEncoding/Probing/FFProbeHelpers.cs:16
static void HarmonizeProbeTags(InternalMediaInfoResult result)
{
    ArgumentNullException.ThrowIfNull(result);

    if (result.Streams is not null)
    {
        // Rewrite every stream's tag dictionary to be case-insensitive.
        foreach (var stream in result.Streams)
        {
            if (stream.Tags is not null)
            {
                stream.Tags = ConvertDictionaryToCaseInsensitive(stream.Tags);
            }
        }
    }

    if (result.Format?.Tags is not null)
    {
        result.Format.Tags = ConvertDictionaryToCaseInsensitive(result.Format.Tags);
    }
}
