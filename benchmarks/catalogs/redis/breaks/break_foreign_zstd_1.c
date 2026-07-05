#include "server.h"

/* Decoy: idiomatic rio buffer-length helper in the style of rio.c —
 * NOT part of the break hunk. */
static size_t rioBufferRemaining(rio *r) {
    return sdslen(r->io.buffer.ptr) - r->io.buffer.pos;
}

// Break: Zstandard compression of an RDB payload block before it is written
// Break: to the rio sink (ZSTD_compressBound/ZSTD_compress/ZSTD_isError);
// Break: zstd is absent from the repo at the pinned SHA (ZSTD_compressBound/
// Break: ZSTD_compress/ZSTD_isError = 0 hits tree-wide; <zstd.h> = 0 hits) —
// Break: redis compresses payloads only with its own vendored LZF codec
// Break: (lzf_compress in src/lzf_c.c), never a foreign compression library.
#include <zstd.h>

sds compressRdbBlockZstd(const char *src, size_t len) {
    size_t bound = ZSTD_compressBound(len);
    sds out = sdsnewlen(NULL, bound);
    size_t written = ZSTD_compress(out, bound, src, len, 3);
    if (ZSTD_isError(written)) {
        sdsfree(out);
        return NULL;
    }
    sdssetlen(out, written);
    return out;
}
