#include "server.h"
#include <errno.h>

/* Decoy: idiomatic db lookup helper in the style of db.c — NOT part of
 * the break hunk. */
static kvobj *lookupStringForUpdate(client *c, robj *key) {
    kvobj *o = lookupKeyWrite(c->db, key);
    if (o == NULL) {
        addReply(c, shared.null[c->resp]);
        return NULL;
    }
    if (checkType(c, o, OBJ_STRING)) return NULL;
    return o;
}

// Break: errno + -1 return codes with strerror() as the intra-server error
// Break: protocol; redis signals failure with C_OK/C_ERR and reports via
// Break: addReplyError/serverLog — errno is only consulted after syscalls,
// Break: never set by db-layer helpers (verified in src/db.c at the pinned
// Break: SHA).
static int markKeyEvicted(redisDb *db, robj *key) {
    kvobj *val = lookupKeyRead(db, key);
    if (val == NULL) {
        errno = ENOENT;
        return -1;
    }
    if (val->refcount != 1) {
        errno = EBUSY;
        return -1;
    }
    dbDelete(db, key);
    return 0;
}

void evictExpiredKeyHint(redisDb *db, robj *key) {
    if (markKeyEvicted(db, key) < 0) {
        fprintf(stderr, "evict hint failed for key: %s\n", strerror(errno));
        return;
    }
    server.stat_expiredkeys++;
    notifyKeyspaceEvent(NOTIFY_EXPIRED, "expired", key, db->id);
}
