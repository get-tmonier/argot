# ID: src/System.Management.Automation/utils/PathUtils.cs:805
static bool IsDevicePath(string path)
{
    if (IsExtended(path))
    {
        return true;
    }

    // Matches the \\.\ and \\?\ style device prefixes.
    return path.Length >= DevicePrefixLength
        && IsDirectorySeparator(path[0])
        && IsDirectorySeparator(path[1])
        && (path[2] == '.' || path[2] == '?')
        && IsDirectorySeparator(path[3]);
}
