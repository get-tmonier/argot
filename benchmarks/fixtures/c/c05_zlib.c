#include <zlib.h>

int deflate_buf(z_stream *s) {
    deflateInit(s, Z_DEFAULT_COMPRESSION);
    return deflate(s, Z_FINISH);
}
