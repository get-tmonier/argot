# ID: MediaBrowser.Common/Net/NetworkUtils.cs:33
static bool IsLinkLocalIPv6(IPAddress address)
{
    ArgumentNullException.ThrowIfNull(address);

    if (address.IsIPv4MappedToIPv6)
    {
        address = address.MapToIPv4();
    }

    if (address.AddressFamily != AddressFamily.InterNetworkV6)
    {
        return false;
    }

    Span<byte> octets = stackalloc byte[16];
    address.TryWriteBytes(octets, out _);

    // fe80::/10 delimits the link-local range.
    uint leadingWord = (uint)(octets[0] << 8) + octets[1];
    return leadingWord >= 0xfe80 && leadingWord <= 0xfebf;
}
