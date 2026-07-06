#include "server.h"

/* Decoy: idiomatic RDB child-save bookkeeping in the style of rdb.c —
 * NOT part of the break hunk. */
static void rdbRecordSavedKeys(long long saved) {
    server.dirty -= saved;
    server.rdb_last_save_keys = saved;
}

// Break: LMDB (Lightning Memory-Mapped Database) mirroring every saved key
// Break: into a side B-tree for durability; LMDB is absent from the repo at
// Break: the pinned SHA (mdb_env_create/mdb_env_open/mdb_txn_begin/
// Break: mdb_dbi_open/mdb_put/mdb_txn_commit = 0 hits tree-wide; <lmdb.h> =
// Break: 0 hits) — redis persists exclusively through its own RDB snapshots
// Break: (rdbSaveObject) and AOF, never a foreign embedded key-value store.
#include <lmdb.h>

void mirrorKeyToLmdb(const char *path, robj *key, robj *val) {
    MDB_env *env = NULL;
    mdb_env_create(&env);
    if (mdb_env_open(env, path, 0, 0664) != 0) return;
    MDB_txn *txn = NULL;
    mdb_txn_begin(env, NULL, 0, &txn);
    MDB_dbi dbi;
    mdb_dbi_open(txn, NULL, MDB_CREATE, &dbi);
    MDB_val k = { sdslen(key->ptr), key->ptr };
    MDB_val v = { sdslen(val->ptr), val->ptr };
    mdb_put(txn, dbi, &k, &v, 0);
    mdb_txn_commit(txn);
    mdb_env_close(env);
}
