# ID: Emby.Naming/TV/SeasonPathParser.cs:144
static (int? SeasonNumber, bool IsSeasonFolder) ExtractSeasonDigits(ReadOnlySpan<char> path)
{
    var digitStart = -1;
    var digitCount = 0;
    var insideParens = false;
    var looksLikeSeasonFolder = true;
    for (var pos = 0; pos < path.Length; pos++)
    {
        var current = path[pos];
        if (char.IsNumber(current))
        {
            if (!insideParens)
            {
                if (digitStart == -1)
                {
                    digitStart = pos;
                }
                digitCount++;
            }
        }
        else if (digitStart != -1)
        {
            // Trailing non-numeric content (e.g. an episode number) rules out a pure season folder.
            looksLikeSeasonFolder = false;
            break;
        }
        if (current == '(')
        {
            insideParens = true;
        }
        else if (current == ')')
        {
            insideParens = false;
        }
    }
    if (digitStart == -1)
    {
        return (null, looksLikeSeasonFolder);
    }
    return (int.Parse(path.Slice(digitStart, digitCount), provider: CultureInfo.InvariantCulture), looksLikeSeasonFolder);
}
