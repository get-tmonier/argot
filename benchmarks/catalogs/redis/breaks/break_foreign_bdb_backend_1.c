#include "server.h"
#include <db.h>

/* Decoy: a redis-style pluggable-backend vtable (mirrors the dictType /
 * moduleType function-pointer idiom) plus an idiomatic cluster helper — NOT
 * part of the break hunk. The foreign <db.h> include above sits in the decoy
 * region, outside the scored hunk. */
typedef struct storageBackend {
    const char *name;
    int (*factory)(DB **, uint32_t);
    int (*put)(DB *, sds, sds);
} storageBackend;

static int clusterCountLocalSlots(void) {
    int n = 0;
    for (int j = 0; j < CLUSTER_SLOTS; j++)
        if (server.cluster->slots[j] == server.cluster->myself) n++;
    return n;
}

// Break: Berkeley DB wired in as a pluggable storage backend through a
// Break: function-pointer vtable — the foreign dependency is masked because
// Break: db_create is taken by address (never called bare) and the write
// Break: goes through the local `dbp` handle's put method, mimicking redis's
// Break: own dictType idiom. Berkeley DB is absent from the repo at the pinned
// Break: SHA (db_create/DB_ENV = 0 hits tree-wide; <db.h> = 0 hits). HARD:
// Break: argot keys on call callees + imports, so a foreign symbol referenced
// Break: only as a function-pointer value may slip past.
static int bdbPut(DB *dbp, sds key, sds val) {
    DBT k, v;
    memset(&k, 0, sizeof(k));
    memset(&v, 0, sizeof(v));
    k.data = key; k.size = sdslen(key);
    v.data = val; v.size = sdslen(val);
    return dbp->put(dbp, NULL, &k, &v, 0);
}

static storageBackend berkeley_backend = {
    .name = "berkeleydb",
    .factory = db_create,
    .put = bdbPut,
};

void registerBerkeleyBackend(void) {
    server.storage_backend = &berkeley_backend;
    serverLog(LL_NOTICE, "registered storage backend %s", berkeley_backend.name);
}
