# ID: MediaBrowser.Controller/Entities/PeopleHelper.cs:89
static void CopyPersonDetails(PersonInfo existing, PersonInfo person)
{
    foreach (var providerId in person.ProviderIds)
    {
        existing.SetProviderId(providerId.Key, providerId.Value);
    }

    existing.ImageUrl = person.ImageUrl ?? existing.ImageUrl;
    existing.SortOrder = person.SortOrder ?? existing.SortOrder;
}
