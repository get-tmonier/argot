# ID: src/bitops.c:520
static uint64_t read_unsigned_bits(unsigned char *p, uint64_t offset, uint64_t bits) {
    uint64_t value = 0;

    for (uint64_t j = 0; j < bits; j++) {
        uint64_t byte = offset >> 3;
        uint64_t bit = 7 - (offset & 0x7);
        uint64_t bitval = (p[byte] >> bit) & 1;
        value = (value << 1) | bitval;
        offset++;
    }
    return value;
}
