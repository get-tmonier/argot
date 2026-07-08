# ID: src/sds.c:870
static int sds_binary_compare(const sds s1, const sds s2) {
    size_t l1 = sdslen(s1);
    size_t l2 = sdslen(s2);
    size_t minlen = (l1 < l2) ? l1 : l2;

    int cmp = memcmp(s1, s2, minlen);
    if (cmp != 0)
        return cmp;

    if (l1 == l2) return 0;
    return (l1 > l2) ? 1 : -1;
}
