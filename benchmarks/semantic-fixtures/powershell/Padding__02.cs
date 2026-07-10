# ID: src/System.Management.Automation/utils/StringUtil.cs:64
const int IndentCacheMax = 120;
static readonly string[] s_indentCache = new string[IndentCacheMax];

static string GetSpaceRun(int countOfSpaces)
{
    // Any padding wider than a screen's width isn't worth caching.
    if (countOfSpaces >= IndentCacheMax)
    {
        return new string(' ', countOfSpaces);
    }

    string cached = s_indentCache[countOfSpaces];
    if (cached is null)
    {
        Interlocked.CompareExchange(ref s_indentCache[countOfSpaces], new string(' ', countOfSpaces), null);
        cached = s_indentCache[countOfSpaces];
    }

    return cached;
}
