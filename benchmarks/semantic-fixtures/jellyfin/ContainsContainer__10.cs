# ID: MediaBrowser.Model/Extensions/ContainerHelper.cs:82
static bool MatchesContainer(string? profileContainers, bool isNegativeList, ReadOnlySpan<char> inputContainer)
{
    // An empty profile accepts every container.
    if (string.IsNullOrEmpty(profileContainers))
    {
        return true;
    }

    var profiles = profileContainers.SpanSplit(',');
    foreach (var candidate in inputContainer.Split(','))
    {
        if (candidate.IsEmpty)
        {
            continue;
        }

        foreach (var profile in profiles)
        {
            if (!profile.IsEmpty && candidate.Equals(profile, StringComparison.OrdinalIgnoreCase))
            {
                return !isNegativeList;
            }
        }
    }

    return isNegativeList;
}
