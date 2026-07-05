#include "server.h"

/* Decoy: idiomatic background-job counter in the style of bio.c —
 * NOT part of the break hunk. */
static void bioTrackPending(int type, long long delta) {
    server.bio_pending[type] += delta;
}

// Break: Grand Central Dispatch fan-out of lazy-free jobs onto a concurrent
// Break: queue (dispatch_get_global_queue/dispatch_group_create/
// Break: dispatch_group_async_f/dispatch_group_wait/dispatch_release); GCD is
// Break: absent from the repo at the pinned SHA (dispatch_get_global_queue/
// Break: dispatch_group_create/dispatch_group_async_f/dispatch_group_wait/
// Break: dispatch_release = 0 hits tree-wide; <dispatch/dispatch.h> = 0 hits)
// Break: — redis offloads background work to its own fixed bio.c thread pool,
// Break: never a foreign concurrency runtime.
#include <dispatch/dispatch.h>

static void freeObjectJob(void *ctx) {
    decrRefCount((robj *)ctx);
}

void fanOutLazyFree(robj **objects, int count) {
    dispatch_queue_t q = dispatch_get_global_queue(DISPATCH_QUEUE_PRIORITY_DEFAULT, 0);
    dispatch_group_t group = dispatch_group_create();
    for (int j = 0; j < count; j++) {
        dispatch_group_async_f(group, q, objects[j], freeObjectJob);
    }
    dispatch_group_wait(group, DISPATCH_TIME_FOREVER);
    dispatch_release(group);
}
