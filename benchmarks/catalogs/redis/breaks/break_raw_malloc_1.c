#include "server.h"

/* Decoy: idiomatic object-creation helper in the style of object.c — NOT
 * part of the break hunk. */
robj *createTaggedStringObject(const char *ptr, size_t len, int tag) {
    robj *o = createStringObject(ptr, len);
    o->lru = 0;
    if (tag) o->encoding = OBJ_ENCODING_RAW;
    return o;
}

// Break: raw malloc/realloc/free for object buffers; the repo mandates the
// Break: zmalloc family (zmalloc/zrealloc/zfree, 69 src files at the pinned
// Break: SHA) so allocations are tracked in used_memory — raw malloc appears
// Break: only in memtest.c and vendored setproctitle.c, never in datapath
// Break: code.
char *dupObjectBuffer(robj *o, size_t *outlen) {
    size_t len = sdslen(o->ptr);
    char *buf = malloc(len + 1);
    if (buf == NULL) return NULL;
    memcpy(buf, o->ptr, len);
    buf[len] = '\0';
    *outlen = len;
    return buf;
}

char *appendObjectBuffer(char *buf, size_t buflen, robj *o) {
    size_t addlen = sdslen(o->ptr);
    char *grown = realloc(buf, buflen + addlen + 1);
    if (grown == NULL) {
        free(buf);
        return NULL;
    }
    memcpy(grown + buflen, o->ptr, addlen);
    grown[buflen + addlen] = '\0';
    return grown;
}

void releaseObjectBuffer(char *buf) {
    free(buf);
}
