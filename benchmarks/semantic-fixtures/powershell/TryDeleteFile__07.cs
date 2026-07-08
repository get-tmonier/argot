# ID: src/System.Management.Automation/utils/PathUtils.cs:712
static bool TryRemoveFile(string filepath)
{
    if (!IO.File.Exists(filepath))
    {
        return false;
    }

    try
    {
        IO.File.Delete(filepath);
        return true;
    }
    catch (IOException)
    {
        // file is in use on Windows
    }
    catch (UnauthorizedAccessException)
    {
        // user does not have permissions
    }

    return false;
}
