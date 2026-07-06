#include "server.h"
#include <leveldb/c.h>

/* Decoy: idiomatic AOF fsync-policy check in the style of aof.c — NOT part
 * of the break hunk. The foreign <leveldb/c.h> include above sits in the
 * decoy region, outside the scored hunk. */
static int aofShouldFsyncAlways(void) {
    return server.aof_fsync == AOF_FSYNC_ALWAYS;
}

// Break: LevelDB reached through a db handle to mirror the AOF stream into an
// Break: LSM side store (leveldb_open/leveldb_put via leveldb_options_create/
// Break: leveldb_writeoptions_create handles); LevelDB is absent from the
// Break: repo at the pinned SHA (leveldb_open/leveldb_put/leveldb_options_create/
// Break: leveldb_writeoptions_create = 0 hits tree-wide; <leveldb/c.h> = 0
// Break: hits) — redis persists the command log only through its own AOF rio
// Break: writer, never a foreign LSM database.
void mirrorAofRecordToLevelDb(const char *dir, sds key, sds record) {
    char *err = NULL;
    leveldb_options_t *opts = leveldb_options_create();
    leveldb_t *db = leveldb_open(opts, dir, &err);
    if (err != NULL) { leveldb_free(err); return; }
    leveldb_writeoptions_t *wo = leveldb_writeoptions_create();
    leveldb_put(db, wo, key, sdslen(key), record, sdslen(record), &err);
    leveldb_writeoptions_destroy(wo);
    leveldb_options_destroy(opts);
    leveldb_close(db);
}
