# ID: src/System.Management.Automation/utils/PsUtils.cs:398
static string EncodeToBase64(string input)
{
    // shell crashes if you pass an empty script block to a native command
    if (input is null)
    {
        throw PSTraceSource.NewArgumentNullException(nameof(input));
    }

    byte[] unicodeBytes = Encoding.Unicode.GetBytes(input.ToCharArray());
    return Convert.ToBase64String(unicodeBytes);
}
