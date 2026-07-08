# ID: src/intset.c:56
static int64_t intset_read_encoded(intset *is, int pos, uint8_t enc) {
    if (enc == INTSET_ENC_INT64) {
        int64_t v64;
        memcpy(&v64, ((int64_t *)is->contents) + pos, sizeof(v64));
        memrev64ifbe(&v64);
        return v64;
    }
    if (enc == INTSET_ENC_INT32) {
        int32_t v32;
        memcpy(&v32, ((int32_t *)is->contents) + pos, sizeof(v32));
        memrev32ifbe(&v32);
        return v32;
    }
    int16_t v16;
    memcpy(&v16, ((int16_t *)is->contents) + pos, sizeof(v16));
    memrev16ifbe(&v16);
    return v16;
}
