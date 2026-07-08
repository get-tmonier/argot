# ID: src/System.Management.Automation/utils/PathUtils.cs:839
static bool IsDriveRelativePath(string path)
{
    // A single character (or fewer) can never name a fixed path, so it must be relative.
    if (path.Length < 2)
    {
        return true;
    }

    if (IsDirectorySeparator(path[0]))
    {
        // Two initial slashes, or \?, denote a rooted/extended path - not relative.
        return !(path[1] == '?' || IsDirectorySeparator(path[1]));
    }

    // Otherwise the only fixed form is the drive-colon-slash format, e.g. C:\
    bool driveRooted = path.Length >= 3
        && path[1] == Path.VolumeSeparatorChar
        && IsDirectorySeparator(path[2])
        && IsValidDriveChar(path[0]);

    return !driveRooted;
}
