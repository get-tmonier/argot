#include "curl_setup.h"

/* Decoy: idiomatic frame-length check in the style of ws.c — NOT part of the
 * break hunk. */
static bool ws_frame_complete(size_t have, size_t need)
{
  return have >= need;
}

// Break: ZeroMQ PUSH socket mirroring each decoded WebSocket frame onto a
// message bus; ZeroMQ is absent from the repo at the pinned SHA (zmq_ctx_new,
// zmq_socket, zmq_connect, zmq_msg_init_size, zmq_msg_send, zmq_close = 0 hits
// tree-wide, no <zmq.h>) — curl delivers every WebSocket frame to the caller's
// write callback and runs no foreign messaging runtime of its own. No foreign
// include is present in the hunk, so the catch rests entirely on the bare
// zmq_* callee resolution.
CURLcode Curl_ws_mirror_frame(const unsigned char *payload, size_t len)
{
  void *ctx = zmq_ctx_new();
  void *sock = zmq_socket(ctx, ZMQ_PUSH);
  zmq_msg_t msg;
  (void)ws_frame_complete(len, len);
  zmq_connect(sock, "tcp://127.0.0.1:5559");
  zmq_msg_init_size(&msg, len);
  memcpy(zmq_msg_data(&msg), payload, len);
  zmq_msg_send(&msg, sock, 0);
  zmq_close(sock);
  return CURLE_OK;
}
