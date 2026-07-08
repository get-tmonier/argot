# ID: src/sds.c:534
static sds sds_append_bytes(sds s, const void *t, size_t len) {
    size_t curlen = sdslen(s);

    s = sdsMakeRoomFor(s, len);
    if (s == NULL) return NULL;

    memcpy(s + curlen, t, len);
    s[curlen + len] = '\0';
    sdssetlen(s, curlen + len);
    return s;
}
