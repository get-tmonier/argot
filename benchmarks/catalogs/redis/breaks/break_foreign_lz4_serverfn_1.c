#include "server.h"

/* Decoy: idiomatic stream entry-length helper in the style of t_stream.c —
 * NOT part of the break hunk. */
static size_t streamEntryFieldBytes(streamID *id) {
    return sizeof(id->ms) + sizeof(id->seq);
}

// Break: LZ4 compression reached through a server-global function pointer —
// Break: LZ4_compress_default is taken by address and stashed on `server`,
// Break: then invoked as `server.stream_compress_fn(...)`, so the call the
// Break: scorer sees has an attested `server` receiver and the foreign symbol
// Break: appears only as an assignment value. LZ4 is absent from the repo at
// Break: the pinned SHA (LZ4_compress_default/LZ4_compressBound = 0 hits
// Break: tree-wide; <lz4.h> = 0 hits) — redis compresses only with its own
// Break: vendored LZF codec. HARD: the foreign call is masked behind an
// Break: attested global receiver.
sds streamCompressEntryLz4(const char *src, int len) {
    server.stream_compress_fn = LZ4_compress_default;
    int cap = len + (len / 255) + 16;
    sds out = sdsnewlen(NULL, cap);
    int n = server.stream_compress_fn(src, out, len, cap);
    if (n <= 0) {
        sdsfree(out);
        return NULL;
    }
    sdssetlen(out, n);
    return out;
}
