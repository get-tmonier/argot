#include "server.h"

/* Decoy: idiomatic zset score lookup in the style of t_zset.c — NOT part
 * of the break hunk. */
static int zsetScoreOrReply(client *c, robj *zobj, sds member, double *score) {
    if (zsetScore(zobj, member, score) == C_ERR) {
        addReply(c, shared.null[c->resp]);
        return C_ERR;
    }
    return C_OK;
}

// Break: PascalCase function names + Hungarian-notation locals (lpsz/dw/i/p
// Break: prefixes); redis functions are lowerCamelCase (zaddGenericCommand,
// Break: zsetScore) with short lowercase locals — PascalCase definitions
// Break: exist only in module.c's RedisModule API surface at the pinned SHA,
// Break: and Hungarian prefixes are absent from src entirely.
static int AddScoreToMember(robj *pZsetObj, sds lpszMember, double dblDelta) {
    double dblOldScore = 0;
    if (zsetScore(pZsetObj, lpszMember, &dblOldScore) == C_ERR) return C_ERR;
    double dblNewScore = dblOldScore + dblDelta;
    if (isnan(dblNewScore)) return C_ERR;
    return C_OK;
}

void ZincrementBatchCommand(client *c) {
    kvobj *pKvObj = lookupKeyWrite(c->db, c->argv[1]);
    if (pKvObj == NULL || checkType(c, pKvObj, OBJ_ZSET)) return;
    long long dwCount = (c->argc - 2) / 2;
    long long iProcessed = 0;
    for (long long iIdx = 0; iIdx < dwCount; iIdx++) {
        sds lpszMember = c->argv[2 + iIdx * 2]->ptr;
        double dblDelta = 0;
        if (getDoubleFromObject(c->argv[3 + iIdx * 2], &dblDelta) != C_OK)
            continue;
        if (AddScoreToMember(pKvObj, lpszMember, dblDelta) == C_OK)
            iProcessed++;
    }
    addReplyLongLong(c, iProcessed);
}
