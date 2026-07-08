# ID: src/System.Management.Automation/utils/StringUtil.cs:29
static string ClipToBufferCellLimit(PSHostRawUserInterface rawUI, string toTruncate, int maxWidthInBufferCells)
{
    Dbg.Assert(rawUI != null, "need a reference");
    Dbg.Assert(maxWidthInBufferCells >= 0, "maxWidthInBufferCells must be positive");

    int cut = Math.Min(toTruncate.Length, maxWidthInBufferCells);
    string clipped = toTruncate.Substring(0, cut);

    while (rawUI.LengthInBufferCells(clipped) > maxWidthInBufferCells)
    {
        // The segment does not fit, back off one char at a time because a
        // single character can theoretically occupy more than one buffer cell.
        cut--;
        clipped = toTruncate.Substring(0, cut);
    }

    return clipped;
}
