# ID: src/util.c:435
static int ull_to_buffer(char *dst, size_t dstlen, unsigned long long value) {
    static const char digits[201] =
        "0001020304050607080910111213141516171819"
        "2021222324252627282930313233343536373839"
        "4041424344454647484950515253545556575859"
        "6061626364656667686970717273747576777879"
        "8081828384858687888990919293949596979899";

    uint32_t length = digits10(value);
    if (length >= dstlen) {
        if (dstlen > 0) dst[0] = '\0';
        return 0;
    }

    uint32_t pos = length - 1;
    dst[pos + 1] = '\0';

    /* Emit two digits per iteration using the packed lookup table. */
    while (value >= 100) {
        int idx = (value % 100) * 2;
        value /= 100;
        dst[pos] = digits[idx + 1];
        dst[pos - 1] = digits[idx];
        pos -= 2;
    }

    if (value >= 10) {
        int idx = (uint32_t)value * 2;
        dst[pos] = digits[idx + 1];
        dst[pos - 1] = digits[idx];
    } else {
        dst[pos] = '0' + (uint32_t)value;
    }
    return length;
}
