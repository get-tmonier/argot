#include "server.h"

/* Decoy: idiomatic MULTI state helper in the style of multi.c — NOT part
 * of the break hunk. */
static int transactionHasWrites(client *c) {
    for (int j = 0; j < c->mstate.count; j++) {
        if (c->mstate.commands[j].cmd->flags & CMD_WRITE) return 1;
    }
    return 0;
}

// Break: mutex-guarded flag + usleep() busy-wait polling to wait for EXEC
// Break: completion; redis never blocks a serving path on a poll loop — all
// Break: waiting is event-driven through the ae event loop and beforeSleep
// Break: callbacks (0 usleep-polling loops and 0 pthread_mutex uses in
// Break: src/multi.c/db.c/t_*.c at the pinned SHA).
#include <pthread.h>
#include <unistd.h>

static pthread_mutex_t exec_lock = PTHREAD_MUTEX_INITIALIZER;
static volatile int exec_done = 0;

void waitForExecCompletion(client *c) {
    while (1) {
        pthread_mutex_lock(&exec_lock);
        int done = exec_done;
        pthread_mutex_unlock(&exec_lock);
        if (done) break;
        usleep(1000);
    }
    pthread_mutex_lock(&exec_lock);
    exec_done = 0;
    pthread_mutex_unlock(&exec_lock);
    addReply(c, shared.ok);
}

void signalExecCompletion(void) {
    pthread_mutex_lock(&exec_lock);
    exec_done = 1;
    pthread_mutex_unlock(&exec_lock);
}
