#include "server.h"
#include <libaio.h>

/* Decoy: idiomatic RDB save-info reset in the style of rdb.c — NOT part of
 * the break hunk. The foreign <libaio.h> include above sits in the decoy
 * region, outside the scored hunk. */
static void rdbResetSaveInfo(void) {
    server.rdb_save_time_last = time(NULL);
    server.rdb_changes_since_save = 0;
}

// Break: Linux libaio submitting RDB writes asynchronously through an io
// Break: context handle (io_setup/io_prep_pwrite/io_submit/io_getevents/
// Break: io_destroy); libaio is absent from the repo at the pinned SHA
// Break: (io_setup/io_prep_pwrite/io_submit/io_getevents/io_destroy = 0 hits
// Break: tree-wide; <libaio.h> = 0 hits) — redis writes the RDB file with its
// Break: own synchronous rio layer and offloads fsync to the bio.c pool,
// Break: never a foreign kernel async-I/O runtime.
void asyncWriteRdbBlock(int fd, void *buf, size_t len, long long off) {
    io_context_t ctx = 0;
    io_setup(8, &ctx);
    struct iocb cb;
    struct iocb *cbs[1] = { &cb };
    io_prep_pwrite(&cb, fd, buf, len, off);
    io_submit(ctx, 1, cbs);
    struct io_event events[1];
    io_getevents(ctx, 1, 1, events, NULL);
    io_destroy(ctx);
}
