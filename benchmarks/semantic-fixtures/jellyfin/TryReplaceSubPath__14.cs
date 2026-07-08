# ID: Emby.Server.Implementations/Library/PathExtensions.cs:95
static bool TrySwapSubPath(string? path, string? subPath, string? newSubPath, out string? newPath)
{
    newPath = null;

    if (string.IsNullOrEmpty(path)
        || string.IsNullOrEmpty(subPath)
        || string.IsNullOrEmpty(newSubPath)
        || subPath.Length > path.Length)
    {
        return false;
    }

    subPath = subPath.NormalizePath(out var separator);
    path = path.NormalizePath(separator);

    if (!path.StartsWith(subPath, StringComparison.OrdinalIgnoreCase))
    {
        return false;
    }

    var subPathEndsWithSeparator = subPath[^1] == separator;
    if (path.Length > subPath.Length
        && !subPathEndsWithSeparator
        && path[subPath.Length] != separator)
    {
        return false;
    }

    var trimmedNewSubPath = newSubPath.AsSpan().TrimEnd(separator);
    var startIndex = subPathEndsWithSeparator ? subPath.Length - 1 : subPath.Length;
    newPath = string.Concat(trimmedNewSubPath, path.AsSpan(startIndex));
    return true;
}
