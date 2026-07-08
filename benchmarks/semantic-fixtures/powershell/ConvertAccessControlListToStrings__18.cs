# ID: src/Microsoft.PowerShell.Commands.Utility/commands/utility/ConvertFrom-SddlString.cs:119
static string[] RenderAclEntries(CommonAcl acl, AccessRightTypeNames? typeName)
{
    if (acl is null || acl.Count == 0)
    {
        return Array.Empty<string>();
    }

    List<string> aceStringList = new(acl.Count);
    foreach (CommonAce ace in acl)
    {
        string ntAccount = ConvertToNTAccount(ace.SecurityIdentifier);
        StringBuilder aceString = new();
        aceString.Append($"{ntAccount}: {ace.AceQualifier}");

        if (ace.AceFlags != AceFlags.None)
        {
            aceString.Append($" {ace.AceFlags}");
        }

        List<string> accessRightList = GetApplicableAccessRights(ace.AccessMask, typeName);
        if (accessRightList.Count > 0)
        {
            string accessRights = string.Join(", ", accessRightList);
            aceString.Append($" ({accessRights})");
        }

        aceStringList.Add(aceString.ToString());
    }

    return aceStringList.ToArray();
}
