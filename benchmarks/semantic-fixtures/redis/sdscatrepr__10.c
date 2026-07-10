# ID: src/sds.c:960
static sds sds_append_escaped(sds s, const char *p, size_t len) {
    s = sdsMakeRoomFor(s, len + 2);
    s = sdscatlen(s, "\"", 1);
    for (size_t i = 0; i < len; i++, p++) {
        switch (*p) {
        case '\\':
        case '"':
            s = sdscatprintf(s, "\\%c", *p);
            break;
        case '\n': s = sdscatlen(s, "\\n", 2); break;
        case '\r': s = sdscatlen(s, "\\r", 2); break;
        case '\t': s = sdscatlen(s, "\\t", 2); break;
        case '\a': s = sdscatlen(s, "\\a", 2); break;
        case '\b': s = sdscatlen(s, "\\b", 2); break;
        default:
            if (isprint(*p))
                s = sdscatlen(s, p, 1);
            else
                s = sdscatprintf(s, "\\x%02x", (unsigned char)*p);
            break;
        }
    }
    return sdscatlen(s, "\"", 1);
}
