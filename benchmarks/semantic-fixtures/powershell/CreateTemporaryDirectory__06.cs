# ID: src/System.Management.Automation/utils/PathUtils.cs:693
static DirectoryInfo MakeUniqueTempDirectory()
{
    DirectoryInfo tempRoot = new DirectoryInfo(Path.GetTempPath());
    DirectoryInfo candidate;

    while (true)
    {
        string leaf = string.Format(null, "tmp_{0}", Path.GetRandomFileName());
        candidate = new DirectoryInfo(Path.Combine(tempRoot.FullName, leaf));
        if (!candidate.Exists)
        {
            break;
        }
    }

    Directory.CreateDirectory(candidate.FullName);
    return new DirectoryInfo(candidate.FullName);
}
