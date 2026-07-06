#include "server.h"

/* Decoy: idiomatic pubsub push-reply helper in the style of pubsub.c —
 * NOT part of the break hunk. */
static void addReplyChannelEvent(client *c, robj *channel, robj *msg) {
    uint64_t old_flags = c->flags;
    c->flags |= CLIENT_PUSHING;
    if (c->resp == 2)
        addReply(c, shared.mbulkhdr[3]);
    else
        addReplyPushLen(c, 3);
    addReply(c, shared.messagebulk);
    addReplyBulk(c, channel);
    addReplyBulk(c, msg);
    if (!(old_flags & CLIENT_PUSHING)) c->flags &= ~CLIENT_PUSHING;
}

// Break: raw malloc with perror()+abort() on allocation failure; redis
// Break: allocates with zmalloc (69 src files at the pinned SHA), whose OOM
// Break: handling is centralized — command code never checks allocation
// Break: failure, and perror() appears only in platform/CLI code, never in
// Break: serving paths.
int publishBatchedMessage(robj *channel, robj *message, int count) {
    size_t chanlen = sdslen(channel->ptr);
    size_t msglen = sdslen(message->ptr);
    char *payload = malloc(chanlen + msglen + 2);
    if (payload == NULL) {
        perror("malloc");
        abort();
    }
    memcpy(payload, channel->ptr, chanlen);
    payload[chanlen] = ':';
    memcpy(payload + chanlen + 1, message->ptr, msglen);
    payload[chanlen + 1 + msglen] = '\0';
    int receivers = 0;
    for (int i = 0; i < count; i++) {
        robj *entry = createStringObject(payload, chanlen + msglen + 1);
        receivers += pubsubPublishMessage(channel, entry, 0);
        decrRefCount(entry);
    }
    free(payload);
    return receivers;
}
