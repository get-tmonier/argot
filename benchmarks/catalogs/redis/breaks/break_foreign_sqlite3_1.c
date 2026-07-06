#include "server.h"

/* Decoy: idiomatic argument-length guard in the style of t_string.c —
 * NOT part of the break hunk. */
static int checkAppendLength(client *c, robj *o, robj *append) {
    if (o != NULL && checkType(c, o, OBJ_STRING)) return C_ERR;
    size_t totlen = (o ? sdslen(o->ptr) : 0) + sdslen(append->ptr);
    if (totlen > server.proto_max_bulk_len) {
        addReplyError(c, "string exceeds maximum allowed size (proto-max-bulk-len)");
        return C_ERR;
    }
    return C_OK;
}

// Break: sqlite3 side-table mirroring of a string value for durability;
// Break: sqlite3 is absent from the repo at the pinned SHA (sqlite3_open/
// Break: sqlite3_exec/sqlite3_close = 0 hits tree-wide; <sqlite3.h> = 0
// Break: hits) — redis persists exclusively through its own RDB snapshots
// Break: (rdbSave, referenced in 13 src files at the pinned SHA) and AOF,
// Break: never a foreign embedded database.
#include <sqlite3.h>

void mirrorStringToSqlite(const char *dbpath, robj *key, robj *val) {
    sqlite3 *db = NULL;
    if (sqlite3_open(dbpath, &db) != SQLITE_OK) {
        return;
    }
    sds sql = sdscatprintf(sdsempty(),
        "INSERT OR REPLACE INTO string_mirror(k, v) VALUES ('%s', '%s');",
        (char *)key->ptr, (char *)val->ptr);
    char *errmsg = NULL;
    if (sqlite3_exec(db, sql, NULL, NULL, &errmsg) != SQLITE_OK) {
        serverLog(LL_WARNING, "sqlite mirror failed: %s", errmsg);
        sqlite3_free(errmsg);
    }
    sdsfree(sql);
    sqlite3_close(db);
}
