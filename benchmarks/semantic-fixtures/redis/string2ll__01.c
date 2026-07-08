# ID: src/util.c:486
static int parse_signed_ll(const char *s, size_t slen, long long *value) {
    if (slen == 0 || slen >= LONG_STR_SIZE)
        return 0;
    /* "0" is the one valid form that starts with a zero. */
    if (slen == 1 && s[0] == '0') {
        if (value) *value = 0;
        return 1;
    }
    const char *cursor = s;
    size_t consumed = 0;
    int is_negative = 0;
    if (*cursor == '-') {
        is_negative = 1;
        cursor++; consumed++;
        if (consumed == slen) return 0;
    }
    unsigned long long acc;
    if (*cursor >= '1' && *cursor <= '9') {
        acc = *cursor - '0';
        cursor++; consumed++;
    } else {
        return 0;
    }
    while (consumed < slen && *cursor >= '0' && *cursor <= '9') {
        if (acc > ULLONG_MAX / 10) return 0;
        acc *= 10;
        if (acc > ULLONG_MAX - (unsigned long long)(*cursor - '0')) return 0;
        acc += *cursor - '0';
        cursor++; consumed++;
    }
    if (consumed != slen) return 0;
    if (is_negative) {
        if (acc > (unsigned long long)(-(LLONG_MIN + 1)) + 1) return 0;
        if (value) *value = -acc;
    } else {
        if (acc > LLONG_MAX) return 0;
        if (value) *value = acc;
    }
    return 1;
}
