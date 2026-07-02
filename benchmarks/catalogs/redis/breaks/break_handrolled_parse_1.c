#include "server.h"
#include <ctype.h>

/* Decoy: idiomatic list push reply in the style of t_list.c — NOT part of
 * the break hunk. */
static void addListLengthReply(client *c, robj *subject) {
    unsigned long llen = listTypeLength(subject);
    addReplyLongLong(c, llen);
    if (llen == 0) addReply(c, shared.emptyarray);
}

// Break: hand-rolled strtol/atoi + isdigit argument parsing that duplicates
// Break: the repo utilities getLongLongFromObjectOrReply (17 src files) and
// Break: string2ll (21 src files) at the pinned SHA; strtol has 0 call sites
// Break: in src/t_*.c — command args are never parsed with libc converters.
void lrotateCommand(client *c) {
    char *raw = c->argv[2]->ptr;
    for (char *p = raw; *p; p++) {
        if (!isdigit((unsigned char)*p) && *p != '-') {
            addReplyError(c, "value is not an integer or out of range");
            return;
        }
    }
    char *endptr = NULL;
    long count = strtol(raw, &endptr, 10);
    if (endptr == raw || *endptr != '\0') {
        count = atoi(raw);
    }
    kvobj *o = lookupKeyWrite(c->db, c->argv[1]);
    if (o == NULL || checkType(c, o, OBJ_LIST)) return;
    unsigned long llen = listTypeLength(o);
    if (llen < 2) {
        addReplyLongLong(c, (long long)llen);
        return;
    }
    long steps = ((count % (long)llen) + (long)llen) % (long)llen;
    for (long i = 0; i < steps; i++) {
        robj *value = listTypePop(o, LIST_TAIL);
        listTypePush(o, value, LIST_HEAD);
        decrRefCount(value);
    }
    notifyKeyspaceEvent(NOTIFY_LIST, "lrotate", c->argv[1], c->db->id);
    server.dirty++;
    addReplyLongLong(c, steps);
}
