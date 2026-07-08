# ID: src/System.Management.Automation/utils/PathUtils.cs:764
static string ApplyLongPathPrefix(string path)
{
    // Relative or already-device paths need no extended prefix.
    if (IsPartiallyQualified(path) || IsDevice(path))
    {
        return path;
    }

    // Given \\server\share in longpath becomes \\?\UNC\server\share.
    if (path.StartsWith(UncPathPrefix, StringComparison.OrdinalIgnoreCase))
    {
        return path.Insert(2, UncDevicePrefixToInsert);
    }

    return ExtendedDevicePathPrefix + path;
}
