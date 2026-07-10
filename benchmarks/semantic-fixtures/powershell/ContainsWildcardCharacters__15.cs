# ID: src/System.Management.Automation/engine/regex.cs:305
static bool HasWildcardCharacters(string pattern)
{
    if (string.IsNullOrEmpty(pattern))
    {
        return false;
    }

    for (int index = 0; index < pattern.Length; ++index)
    {
        if (IsWildcardChar(pattern[index]))
        {
            return true;
        }

        // If it is an escape character then advance past the next character.
        if (pattern[index] == escapeChar)
        {
            ++index;
        }
    }

    return false;
}
