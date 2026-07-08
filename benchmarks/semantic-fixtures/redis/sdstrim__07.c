# ID: src/sds.c:783
static sds sds_strip_charset(sds s, const char *cset) {
    char *start = s;
    char *last = s + sdslen(s) - 1;
    char *finish = last;

    while (start <= last && strchr(cset, *start)) start++;
    while (finish > start && strchr(cset, *finish)) finish--;

    size_t len = (finish - start) + 1;
    if (start != s) memmove(s, start, len);
    s[len] = '\0';
    sdssetlen(s, len);
    return s;
}
