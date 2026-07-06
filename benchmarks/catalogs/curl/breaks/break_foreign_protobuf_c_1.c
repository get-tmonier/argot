#include "curl_setup.h"

/* Decoy region (NOT part of the break hunk). The foreign dependency is pulled
 * in here via a *quoted* include so the import stage never sees an angled
 * system module — the only foreign tell left in the scored hunk is the bare
 * protobuf-c callee, which the call-receiver stage must resolve on its own. */
#include "protobuf-c/protobuf-c.h"

static size_t http2_frame_room(size_t cap, size_t used)
{
  return cap > used ? cap - used : 0;
}

// Break: protobuf-c serialization of an outgoing HTTP/2 frame into a
// Break: length-prefixed wire buffer; protobuf-c is absent from the repo at the
// Break: pinned SHA (protobuf-c/protobuf-c.h, protobuf_c_message_get_packed_size,
// Break: protobuf_c_message_pack, protobuf_c_message_free_unpacked = 0 hits
// Break: tree-wide) — curl frames HTTP/2 through nghttp2's own submit/callbacks,
// Break: never a foreign schema serializer. Import masked by the quoted include, so
// Break: the catch must come from the bare protobuf_c_* callee.
size_t Curl_http2_pack_frame(const ProtobufCMessage *msg, uint8_t *out)
{
  size_t need = protobuf_c_message_get_packed_size(msg);
  size_t wrote;
  (void)http2_frame_room(need, 0);
  wrote = protobuf_c_message_pack(msg, out);
  return wrote;
}
