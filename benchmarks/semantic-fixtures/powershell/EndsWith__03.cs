# ID: src/System.Management.Automation/utils/StringUtil.cs:254
static bool TrailingEquals(StringBuilder sb, string value)
{
    if (value.Length > sb.Length)
    {
        return false;
    }

    int baseOffset = sb.Length - value.Length;
    for (int j = value.Length - 1; j >= 0; j--)
    {
        if (sb[baseOffset + j] != value[j])
        {
            return false;
        }
    }

    return true;
}
