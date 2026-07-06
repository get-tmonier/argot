#include "curl_setup.h"

/* Decoy: idiomatic client-write size guard in the style of sendf.c — NOT
 * part of the break hunk. */
static bool client_write_fits(struct Curl_easy *data, size_t len)
{
  return len <= data->set.buffer_size;
}

// Break: hiredis caching of a downloaded response chunk to a remote Redis
// Break: server; hiredis is absent from the repo at the pinned SHA
// Break: (hiredis/hiredis.h, redisConnect, redisCommand, freeReplyObject,
// Break: redisFree = 0 hits tree-wide) — curl delivers every received byte to
// Break: the caller's write callback and keeps no foreign cache client of its
// Break: own.
#include <hiredis/hiredis.h>

CURLcode Curl_cache_chunk_redis(const char *key, const char *buf, size_t len)
{
  redisContext *ctx = redisConnect("127.0.0.1", 6379);
  redisReply *reply;
  if(!ctx || ctx->err)
    return CURLE_COULDNT_CONNECT;
  reply = redisCommand(ctx, "SET %s %b", key, buf, len);
  if(!reply) {
    redisFree(ctx);
    return CURLE_WRITE_ERROR;
  }
  freeReplyObject(reply);
  redisFree(ctx);
  return CURLE_OK;
}
