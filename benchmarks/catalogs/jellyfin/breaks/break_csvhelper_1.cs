    // Break: fixture spliced at class-member level into Controllers/LibraryController.cs.
    // Break: decoy below mirrors the host's own inline string assembly; the hunk does not.

    /// <summary>
    /// Joins the physical library paths into a newline-delimited body the way this
    /// controller already builds simple text responses — by hand.
    /// </summary>
    private static string JoinPhysicalPaths(IEnumerable<string> paths)
    {
        return string.Join(Environment.NewLine, paths);
    }

    // Break: begin hunk — CsvHelper CsvWriter serialises the physical-path export instead of the
    // Break: hand-written join above. CsvHelper is 0-usage in the repo at the pinned SHA — the API
    // Break: layer emits responses through System.Text.Json and inline string building, never a CSV lib.
    using CsvHelper;
    private static string WritePhysicalPathsCsv(IEnumerable<string> paths)
    {
        using var buffer = new StringWriter();
        using var csv = new CsvWriter(buffer, CultureInfo.InvariantCulture);
        csv.WriteRecords(paths);
        return buffer.ToString();
    }
    // Break: end hunk

    /// <summary>
    /// True when the caller is allowed to enumerate raw physical library paths.
    /// </summary>
    private bool CanListPhysicalPaths()
        => User.IsInRole("Administrator");
