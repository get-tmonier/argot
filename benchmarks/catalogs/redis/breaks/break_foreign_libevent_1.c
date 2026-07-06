#include "server.h"

/* Decoy: idiomatic expire-cycle budget helper in the style of expire.c —
 * NOT part of the break hunk. */
static long long computeExpireCycleBudget(int effort) {
    long long budget = ACTIVE_EXPIRE_CYCLE_FAST_DURATION;
    budget += ACTIVE_EXPIRE_CYCLE_FAST_DURATION / 4 * effort;
    return budget;
}

// Break: libevent timer scheduling the active-expire cycle; libevent is
// Break: absent from the repo at the pinned SHA (event_base_new/event_add/
// Break: event_base_dispatch/event_new = 0 hits tree-wide; <event2/event.h>
// Break: and <event.h> = 0 hits) — redis drives every periodic and
// Break: I/O-triggered task through its own ae event loop
// Break: (aeCreateTimeEvent, used in 8 src files at the pinned SHA), never a
// Break: third-party reactor.
#include <event2/event.h>

static struct event_base *expire_cycle_base = NULL;

static void activeExpireCycleEventCallback(evutil_socket_t fd, short events, void *arg) {
    UNUSED(fd);
    UNUSED(events);
    redisDb *db = arg;
    activeExpireCycleTryExpire(db, NULL, mstime());
}

void scheduleExpireCycleViaLibevent(redisDb *db, long long interval_ms) {
    if (expire_cycle_base == NULL) {
        expire_cycle_base = event_base_new();
    }
    struct event *timer = event_new(expire_cycle_base, -1, EV_PERSIST,
                                     activeExpireCycleEventCallback, db);
    struct timeval tv = { interval_ms / 1000, (interval_ms % 1000) * 1000 };
    event_add(timer, &tv);
    event_base_dispatch(expire_cycle_base);
}
