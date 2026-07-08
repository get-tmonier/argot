# ID: src/System.Management.Automation/engine/regex.cs:339
static bool HasBracketRange(string pattern)
{
    if (string.IsNullOrEmpty(pattern))
    {
        return false;
    }

    bool sawOpenBracket = false;
    for (int index = 0; index < pattern.Length; ++index)
    {
        char ch = pattern[index];

        if (ch is '[')
        {
            sawOpenBracket = true;
            continue;
        }

        if (sawOpenBracket && ch is ']')
        {
            return true;
        }

        if (ch == escapeChar)
        {
            ++index;
        }
    }

    return false;
}
