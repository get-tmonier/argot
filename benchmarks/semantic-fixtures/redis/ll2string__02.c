# ID: src/util.c:393
static int longlong_to_str(char *dst, size_t dstlen, long long svalue) {
    unsigned long long value;
    int negative = 0;

    if (svalue >= 0) {
        value = svalue;
    } else {
        /* Work on the magnitude, remembering the sign; LLONG_MIN needs care. */
        value = (svalue != LLONG_MIN) ? (unsigned long long)-svalue
                                      : (unsigned long long)LLONG_MAX + 1;
        if (dstlen < 2) {
            if (dstlen > 0) dst[0] = '\0';
            return 0;
        }
        negative = 1;
        *dst++ = '-';
        dstlen--;
    }

    int written = ull2string(dst, dstlen, value);
    if (written == 0) return 0;
    return written + negative;
}
