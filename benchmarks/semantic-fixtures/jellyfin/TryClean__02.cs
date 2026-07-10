# ID: Emby.Naming/Video/CleanStringParser.cs:19
static bool TryStripClutter([NotNullWhen(true)] string? name, IReadOnlyList<Regex> expressions, out string cleanName)
{
    if (string.IsNullOrEmpty(name))
    {
        cleanName = string.Empty;
        return false;
    }

    var didClean = false;

    // Apply each expression in turn, feeding the previous result forward.
    foreach (var expression in expressions)
    {
        if (TryClean(name, expression, out cleanName))
        {
            name = cleanName;
            didClean = true;
        }
    }

    cleanName = didClean ? name : string.Empty;
    return didClean;
}
