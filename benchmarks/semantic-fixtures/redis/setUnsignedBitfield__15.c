# ID: src/bitops.c:500
static void write_unsigned_bits(unsigned char *p, uint64_t offset, uint64_t bits, uint64_t value) {
    for (uint64_t j = 0; j < bits; j++) {
        uint64_t bitval = (value & ((uint64_t)1 << (bits - 1 - j))) != 0;
        uint64_t byte = offset >> 3;
        uint64_t bit = 7 - (offset & 0x7);
        uint64_t byteval = p[byte];
        byteval &= ~(1 << bit);
        byteval |= bitval << bit;
        p[byte] = byteval & 0xff;
        offset++;
    }
}
