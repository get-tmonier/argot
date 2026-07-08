# ID: src/Microsoft.PowerShell.Commands.Utility/commands/utility/ConvertFrom-SddlString.cs:61
static List<string> CollectMatchingAccessRights(int accessMask, AccessRightTypeNames? typeName)
{
    List<Type> typesToExamine = new();
    List<string> foundAccessRightNames = new();
    HashSet<int> foundAccessRightValues = new();

    if (typeName is null)
    {
        foreach (AccessRightTypeNames member in Enum.GetValues<AccessRightTypeNames>())
        {
            typesToExamine.Add(GetRealAccessRightType(member));
        }
    }
    else
    {
        typesToExamine.Add(GetRealAccessRightType(typeName.Value));
    }

    foreach (Type accessRightType in typesToExamine)
    {
        foreach (string memberName in Enum.GetNames(accessRightType))
        {
            int memberValue = (int)Enum.Parse(accessRightType, memberName);
            if (foundAccessRightValues.Add(memberValue) && (accessMask & memberValue) == memberValue)
            {
                foundAccessRightNames.Add(memberName);
            }
        }
    }

    foundAccessRightNames.Sort(StringComparer.OrdinalIgnoreCase);
    return foundAccessRightNames;
}
