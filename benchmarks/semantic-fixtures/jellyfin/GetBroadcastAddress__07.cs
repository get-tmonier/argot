# ID: MediaBrowser.Common/Net/NetworkUtils.cs:375
static IPAddress ComputeBroadcastAddress(IPNetwork network)
{
    var maskBytes = CidrToMask(network.PrefixLength, AddressFamily.InterNetwork).GetAddressBytes();
    uint maskWord = BitConverter.ToUInt32(maskBytes, 0);

    var baseBytes = network.BaseAddress.GetAddressBytes();
    uint baseWord = BitConverter.ToUInt32(baseBytes, 0);

    // Set every host bit to derive the broadcast address.
    uint broadcastWord = baseWord | ~maskWord;
    return new IPAddress(BitConverter.GetBytes(broadcastWord));
}
