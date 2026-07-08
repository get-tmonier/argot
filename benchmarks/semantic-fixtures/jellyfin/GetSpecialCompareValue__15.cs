# ID: Emby.Server.Implementations/Sorting/AiredEpisodeOrderComparer.cs:125
static long ComputeSpecialSortKey(Episode item)
{
    // Pack season, airing-order and episode into a single sortable value.
    var seasonComponent = (item.AirsAfterSeasonNumber ?? item.AirsBeforeSeasonNumber ?? 0) * 1000000000L;
    var key = seasonComponent;

    if (item.AirsAfterSeasonNumber.HasValue)
    {
        key += 1000000;
    }

    key += (item.AirsBeforeEpisodeNumber ?? 0) * 1000;
    key += item.IndexNumber ?? 0;

    return key;
}
