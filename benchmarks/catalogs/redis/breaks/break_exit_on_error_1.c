#include "server.h"

/* Decoy: idiomatic argument-check helper in the style of t_string.c —
 * NOT part of the break hunk. */
static int checkRangeArgs(client *c, long long start, long long end) {
    if (start > end) {
        addReplyError(c, "start must be less than or equal to end");
        return C_ERR;
    }
    if (end - start > server.proto_max_bulk_len) {
        addReplyError(c, "requested range exceeds proto-max-bulk-len");
        return C_ERR;
    }
    return C_OK;
}

// Break: fprintf(stderr) + exit(1) on bad client input inside a command
// Break: implementation; redis command paths reply with addReplyError and
// Break: never terminate the process on user error (0 exit()/fprintf(stderr)
// Break: sites in src/t_*.c at the pinned SHA).
void strrotateCommand(client *c) {
    long long count;
    kvobj *o = lookupKeyWrite(c->db, c->argv[1]);
    if (o == NULL) {
        fprintf(stderr, "strrotate: no such key\n");
        exit(1);
    }
    if (checkType(c, o, OBJ_STRING)) {
        fprintf(stderr, "strrotate: WRONGTYPE operation\n");
        exit(EXIT_FAILURE);
    }
    if (getLongLongFromObject(c->argv[2], &count) != C_OK) {
        fprintf(stderr, "strrotate: count is not an integer\n");
        exit(1);
    }
    sds val = o->ptr;
    size_t len = sdslen(val);
    if (len == 0) {
        addReply(c, shared.czero);
        return;
    }
    long long offset = ((count % (long long)len) + (long long)len) % (long long)len;
    sds rotated = sdsnewlen(NULL, len);
    memcpy(rotated, val + offset, len - offset);
    memcpy(rotated + (len - offset), val, offset);
    addReplyBulkSds(c, rotated);
}
