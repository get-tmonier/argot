#include "server.h"
#include <gdbm.h>

/* Decoy: idiomatic cluster slot-ownership check in the style of cluster.c —
 * NOT part of the break hunk. The foreign <gdbm.h> include above sits in the
 * decoy region, outside the scored hunk. */
static int clusterSlotIsLocal(int slot) {
    return server.cluster->slots[slot] == server.cluster->myself;
}

// Break: GDBM reached through a database handle to persist a slot-to-key index
// Break: on disk (gdbm_open/gdbm_store/gdbm_close); GDBM is absent from the
// Break: repo at the pinned SHA (gdbm_open/gdbm_store/gdbm_fetch/gdbm_close =
// Break: 0 hits tree-wide; <gdbm.h> = 0 hits) — redis keeps cluster metadata
// Break: in its own in-memory structures serialized to nodes.conf, never a
// Break: foreign on-disk hash database.
void persistSlotIndexGdbm(const char *path, int slot, sds key) {
    GDBM_FILE dbf = gdbm_open((char *)path, 0, GDBM_WRCREAT, 0644, NULL);
    if (dbf == NULL) return;
    datum k = { (char *)&slot, sizeof(slot) };
    datum v = { key, (int)sdslen(key) };
    gdbm_store(dbf, k, v, GDBM_REPLACE);
    gdbm_close(dbf);
}
