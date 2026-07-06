#include "server.h"

/* Decoy: idiomatic keyspace-notification flag check in the style of
 * notify.c — NOT part of the break hunk. */
static int notifyFlagEnabled(int type) {
    return (server.notify_keyspace_events & type) != 0;
}

// Break: ZeroMQ publishing every keyspace event onto a PUB socket for an
// Break: external subscriber (zmq_ctx_new/zmq_socket/zmq_bind/zmq_send/
// Break: zmq_close); ZeroMQ is absent from the repo at the pinned SHA
// Break: (zmq_ctx_new/zmq_socket/zmq_bind/zmq_send/zmq_close = 0 hits
// Break: tree-wide; <zmq.h> = 0 hits) — redis fans out keyspace events only
// Break: through its own pub/sub (notifyKeyspaceEvent), never a foreign
// Break: message-queue transport.
#include <zmq.h>

static void *event_zmq_ctx = NULL;
static void *event_zmq_pub = NULL;

void publishKeyspaceEventZmq(const char *endpoint, robj *event, robj *key) {
    if (event_zmq_ctx == NULL) {
        event_zmq_ctx = zmq_ctx_new();
        event_zmq_pub = zmq_socket(event_zmq_ctx, ZMQ_PUB);
        zmq_bind(event_zmq_pub, endpoint);
    }
    zmq_send(event_zmq_pub, event->ptr, sdslen(event->ptr), ZMQ_SNDMORE);
    zmq_send(event_zmq_pub, key->ptr, sdslen(key->ptr), 0);
}
