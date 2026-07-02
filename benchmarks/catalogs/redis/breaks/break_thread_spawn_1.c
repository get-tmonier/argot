#include "server.h"

/* Decoy: idiomatic lazy-free accounting helper in the style of
 * lazyfree.c — NOT part of the break hunk. */
static size_t lazyfreeObjectEffort(robj *obj) {
    if (obj->type == OBJ_LIST && obj->encoding == OBJ_ENCODING_QUICKLIST)
        return quicklistCount(obj->ptr);
    if (obj->type == OBJ_SET && obj->encoding == OBJ_ENCODING_HT)
        return dictSize((dict *)obj->ptr);
    return 1;
}

// Break: spawning a detached pthread per object to free it asynchronously;
// Break: redis is a single-threaded event loop (ae.c) and offloads frees to
// Break: the fixed bio.c worker pool via bioCreateLazyFreeJob (verified at
// Break: src/lazyfree.c:234 at the pinned SHA) — pthread_create appears only
// Break: in bio.c/iothread.c infrastructure, never in serving paths.
#include <pthread.h>

static void *freeObjectWorker(void *arg) {
    robj *obj = arg;
    decrRefCount(obj);
    return NULL;
}

void freeObjectInThread(robj *obj) {
    pthread_t tid;
    if (lazyfreeObjectEffort(obj) < 64) {
        decrRefCount(obj);
        return;
    }
    if (pthread_create(&tid, NULL, freeObjectWorker, obj) != 0) {
        decrRefCount(obj);
        return;
    }
    pthread_detach(tid);
}
