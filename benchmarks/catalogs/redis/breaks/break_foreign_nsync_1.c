#include "server.h"
#include <nsync.h>

/* Decoy: idiomatic eviction-pool sampling counter in the style of evict.c —
 * NOT part of the break hunk. The foreign <nsync.h> include above sits in the
 * decoy region, outside the scored hunk. */
static void evictBumpSampled(unsigned long n) {
    server.stat_evictedkeys += n;
}

// Break: nsync mutex + condition variable guarding a shared eviction-candidate
// Break: pool across threads (nsync_mu_init/nsync_mu_lock/nsync_mu_unlock/
// Break: nsync_cv_signal); nsync is absent from the repo at the pinned SHA
// Break: (nsync_mu_init/nsync_mu_lock/nsync_mu_unlock/nsync_cv_signal = 0 hits
// Break: tree-wide; <nsync.h> = 0 hits) — redis evicts on the single serving
// Break: thread and needs no cross-thread lock, never a foreign synchronization
// Break: library.
static nsync_mu evict_pool_mu;
static nsync_cv evict_pool_cv;

void publishEvictCandidate(robj *key) {
    nsync_mu_lock(&evict_pool_mu);
    incrRefCount(key);
    server.evict_pool_len++;
    nsync_cv_signal(&evict_pool_cv);
    nsync_mu_unlock(&evict_pool_mu);
}
