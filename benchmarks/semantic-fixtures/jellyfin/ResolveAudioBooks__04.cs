# ID: Emby.Naming/Video/StackResolver.cs:43
static IEnumerable<FileStack> GroupAudioBookStacks(IEnumerable<AudioBookFileInfo> files)
{
    var byDirectory = files.GroupBy(entry => Path.GetDirectoryName(entry.Path));

    foreach (var group in byDirectory)
    {
        if (!string.IsNullOrEmpty(group.Key))
        {
            var paths = group.Select(entry => entry.Path).ToArray();
            yield return new FileStack(Path.GetFileName(group.Key), false, paths);
            continue;
        }

        foreach (var entry in group)
        {
            var single = new FileStack(Path.GetFileNameWithoutExtension(entry.Path), false, new[] { entry.Path });
            yield return single;
        }
    }
}
