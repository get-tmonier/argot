#include "server.h"

/* Decoy: idiomatic pubsub subscriber-count helper in the style of
 * pubsub.c — NOT part of the break hunk. */
static unsigned long pubsubChannelSubscriberCount(robj *channel) {
    dict *clients = kvstoreDictFetchValue(server.pubsub_channels, 0, channel);
    if (clients == NULL) return 0;
    return dictSize(clients);
}

// Break: jansson JSON encoding of a message envelope before publish;
// Break: jansson is absent from the repo at the pinned SHA
// Break: (json_object/json_object_set_new/json_string/json_dumps/
// Break: json_decref = 0 hits tree-wide; <jansson.h> = 0 hits) — redis
// Break: publishes the raw message bulk string as-is through
// Break: pubsubPublishMessage (src/pubsub.c:533), never wrapping it in a
// Break: serialized envelope of its own.
#include <jansson.h>

int publishJsonAuditEvent(robj *channel, robj *message, long long ts) {
    json_t *envelope = json_object();
    json_object_set_new(envelope, "channel", json_string(channel->ptr));
    json_object_set_new(envelope, "payload", json_string(message->ptr));
    json_object_set_new(envelope, "ts", json_integer(ts));

    char *encoded = json_dumps(envelope, JSON_COMPACT);
    robj *wrapped = createStringObject(encoded, strlen(encoded));
    int receivers = pubsubPublishMessage(channel, wrapped, 0);

    decrRefCount(wrapped);
    free(encoded);
    json_decref(envelope);
    return receivers;
}
