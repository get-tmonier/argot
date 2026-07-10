# ID: src/System.Management.Automation/engine/lang/codegen.cs:21
static string EscapeForSingleQuotes(string value)
{
    if (string.IsNullOrEmpty(value))
    {
        return string.Empty;
    }

    StringBuilder builder = new StringBuilder(value.Length);
    for (int i = 0; i < value.Length; i++)
    {
        char c = value[i];
        builder.Append(c);

        // double-up quotes to escape them
        if (CharExtensions.IsSingleQuote(c))
        {
            builder.Append(c);
        }
    }

    return builder.ToString();
}
