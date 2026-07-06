#include "server.h"
#include <snappy-c.h>

/* Decoy: idiomatic rio checksum update in the style of rio.c — NOT part of
 * the break hunk. The foreign <snappy-c.h> include above sits in the decoy
 * region, outside the scored hunk. */
static void rioUpdateChecksum(rio *r, const void *buf, size_t len) {
    r->cksum = crc64(r->cksum, buf, len);
}

// Break: Snappy compression of a value buffer before it is streamed through
// Break: rio (snappy_compress sized by snappy_max_compressed_length); Snappy
// Break: is absent from the repo at the pinned SHA (snappy_compress/
// Break: snappy_max_compressed_length = 0 hits tree-wide; <snappy-c.h> = 0
// Break: hits) — redis compresses only with its own vendored LZF codec
// Break: (lzf_compress), never a foreign compression library.
sds compressValueSnappy(const char *src, size_t len) {
    size_t out_len = snappy_max_compressed_length(len);
    sds out = sdsnewlen(NULL, out_len);
    if (snappy_compress(src, len, out, &out_len) != SNAPPY_OK) {
        sdsfree(out);
        return NULL;
    }
    sdssetlen(out, out_len);
    return out;
}
