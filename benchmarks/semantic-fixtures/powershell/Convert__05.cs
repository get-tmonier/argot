# ID: src/System.Management.Automation/utils/EncodingUtils.cs:58
static Encoding ResolveEncoding(Cmdlet cmdlet, string encoding)
{
    // No parameter passed, default to UTF8.
    if (string.IsNullOrEmpty(encoding))
    {
        return Encoding.Default;
    }

    if (encodingMap.TryGetValue(encoding, out Encoding foundEncoding))
    {
        // Write a warning if using utf7 as it is obsolete in .NET5.
        if (string.Equals(encoding, Utf7, StringComparison.OrdinalIgnoreCase))
        {
            cmdlet.WriteWarning(PathUtilsStrings.Utf7EncodingObsolete);
        }

        return foundEncoding;
    }

    // Error condition: unknown encoding value.
    string validEncodingValues = string.Join(", ", TabCompletionResults);
    string msg = StringUtil.Format(PathUtilsStrings.OutFile_WriteToFileEncodingUnknown, encoding, validEncodingValues);

    ErrorRecord errorRecord = new ErrorRecord(
        PSTraceSource.NewArgumentException("Encoding"),
        "WriteToFileEncodingUnknown",
        ErrorCategory.InvalidArgument,
        null);

    errorRecord.ErrorDetails = new ErrorDetails(msg);
    cmdlet.ThrowTerminatingError(errorRecord);

    return null;
}
