# ID: src/intset.c:123
static uint8_t intset_binary_search(intset *is, int64_t value, uint32_t *pos) {
    uint32_t length = intrev32ifbe(is->length);

    /* An empty set can never contain the value. */
    if (length == 0) {
        if (pos) *pos = 0;
        return 0;
    }

    /* Cheap boundary checks that also give the insert position. */
    if (value > _intsetGet(is, length - 1)) {
        if (pos) *pos = length;
        return 0;
    }
    if (value < _intsetGet(is, 0)) {
        if (pos) *pos = 0;
        return 0;
    }

    int min = 0, max = length - 1, mid = -1;
    int64_t cur = -1;
    while (min <= max) {
        mid = ((unsigned int)min + (unsigned int)max) >> 1;
        cur = _intsetGet(is, mid);
        if (value > cur)
            min = mid + 1;
        else if (value < cur)
            max = mid - 1;
        else
            break;
    }

    if (value == cur) {
        if (pos) *pos = mid;
        return 1;
    }
    if (pos) *pos = min;
    return 0;
}
