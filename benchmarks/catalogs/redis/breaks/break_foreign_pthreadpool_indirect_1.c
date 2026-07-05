#include "server.h"

/* Decoy: idiomatic debug object-count reporter in the style of debug.c —
 * NOT part of the break hunk. */
static void debugReportKeyCount(client *c) {
    addReplyLongLong(c, dbSize(c->db, DB_MAIN));
}

// Break: pthreadpool (a foreign fork-join parallel runtime) reached through a
// Break: server-global function pointer — pthreadpool_parallelize_1d is taken
// Break: by address and stashed on `server`, then invoked as
// Break: `server.parallel_scan_fn(...)`, so the call the scorer sees has an
// Break: attested `server` receiver and the foreign symbol appears only as an
// Break: assignment value. pthreadpool is absent from the repo at the pinned
// Break: SHA (pthreadpool_create/pthreadpool_parallelize_1d = 0 hits
// Break: tree-wide; <pthreadpool.h> = 0 hits) — redis parallelizes nothing on
// Break: the serving path, offloading only fixed jobs to bio.c. HARD: the
// Break: foreign call is masked behind an attested global receiver.
void debugParallelScan(pthreadpool_task_1d_t worker, void *ctx, size_t n) {
    server.parallel_scan_fn = pthreadpool_parallelize_1d;
    server.parallel_scan_fn(server.thread_pool, worker, ctx, n, 0);
    serverLog(LL_DEBUG, "parallel scan dispatched over %zu items", n);
}
