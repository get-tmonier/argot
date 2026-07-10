# ID: src/util.c:358
static uint32_t count_decimal_digits(uint64_t v) {
    if (v < 10) return 1;
    if (v < 100) return 2;
    if (v < 1000) return 3;
    if (v >= 1000000000000UL)
        return 12 + count_decimal_digits(v / 1000000000000UL);
    if (v < 100000000UL) {
        if (v >= 1000000)
            return 7 + (v >= 10000000UL);
        if (v >= 10000)
            return 5 + (v >= 100000);
        return 4;
    }
    if (v < 10000000000UL)
        return 9 + (v >= 1000000000UL);
    return 11 + (v >= 100000000000UL);
}
