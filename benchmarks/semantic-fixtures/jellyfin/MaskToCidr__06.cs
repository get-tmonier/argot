# ID: MediaBrowser.Common/Net/NetworkUtils.cs:93
static byte CountMaskBits(IPAddress mask)
{
    ArgumentNullException.ThrowIfNull(mask);

    byte prefix = 0;
    if (mask.Equals(IPAddress.Any))
    {
        return prefix;
    }

    Span<byte> octets = stackalloc byte[mask.AddressFamily == AddressFamily.InterNetwork ? NetworkConstants.IPv4MaskBytes : NetworkConstants.IPv6MaskBytes];
    if (!mask.TryWriteBytes(octets, out var written))
    {
        Console.WriteLine("Unable to write address bytes, only {0} bytes written.", written.ToString(CultureInfo.InvariantCulture));
    }

    var seenZero = false;
    foreach (var octet in octets)
    {
        for (int bits = octet; (bits & 0xFF) != 0; bits <<= 1)
        {
            if (seenZero)
            {
                // A set bit following a zero bit means the mask is not contiguous.
                return (byte)~prefix;
            }

            if ((bits & 0x80) == 0)
            {
                seenZero = true;
            }
            else
            {
                prefix++;
            }
        }
    }

    return prefix;
}
