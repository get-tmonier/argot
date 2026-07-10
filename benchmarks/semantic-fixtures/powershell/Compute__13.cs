# ID: src/System.Management.Automation/utils/PsUtils.cs:509
static uint CalculateCrc32(byte[] buffer)
{
    uint crc = 0xFFFFFFFF;
    foreach (byte value in buffer)
    {
        byte slot = (byte)(crc ^ value & 0xff);
        crc = (crc >> 8) ^ table[slot];
    }

    return ~crc;
}
