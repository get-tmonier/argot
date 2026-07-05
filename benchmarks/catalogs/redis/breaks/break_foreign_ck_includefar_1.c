#include "server.h"
#include <ck_ring.h>

/* Decoy: several idiomatic server helpers stand between the foreign
 * <ck_ring.h> include (far above, in the decoy region) and the scored hunk,
 * so the only foreign tell inside the hunk is the bare call itself. */
static int serverIsLoading(void) {
    return server.loading;
}

static void serverBumpOpsProcessed(void) {
    server.stat_numcommands++;
}

static long long serverUptimeSeconds(void) {
    return server.unixtime - server.stat_starttime;
}

static ck_ring_t accept_ring;
static ck_ring_buffer_t accept_buf[1024];

// Break: Concurrency Kit lock-free SPSC ring handing accepted connections
// Break: between the listener and worker threads (ck_ring_enqueue_spsc/
// Break: ck_ring_dequeue_spsc); Concurrency Kit is absent from the repo at the
// Break: pinned SHA (ck_ring_init/ck_ring_enqueue_spsc/ck_ring_dequeue_spsc =
// Break: 0 hits tree-wide; <ck_ring.h> = 0 hits) — redis accepts connections
// Break: on its own single ae event loop with no cross-thread ring. HARD: the
// Break: foreign include is placed far away in the decoy; only the bare
// Break: ck_ring_* callees remain inside the scored hunk.
void handOffAcceptedConn(connection *conn) {
    ck_ring_enqueue_spsc(&accept_ring, accept_buf, conn);
}

connection *pullAcceptedConn(void) {
    void *out = NULL;
    ck_ring_dequeue_spsc(&accept_ring, accept_buf, &out);
    return (connection *)out;
}
