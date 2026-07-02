#include "server.h"

/* Decoy: idiomatic expire-cycle bookkeeping helper in the style of
 * expire.c — NOT part of the break hunk. */
static void updateExpiredStalePerc(long long sampled, long long expired) {
    double current_perc = 0;
    if (sampled) current_perc = (double)expired / sampled;
    server.stat_expired_stale_perc = (current_perc * 0.05) +
                                     (server.stat_expired_stale_perc * 0.95);
}

// Break: printf/fprintf(stdout) diagnostics in a server cycle; redis logs
// Break: through serverLog(LL_VERBOSE/LL_WARNING, ...) (29 src files at the
// Break: pinned SHA) and never writes progress lines to stdout in serving
// Break: paths.
void reportExpireCycle(redisDb *db, long long start_ms, long long expired) {
    long long now = mstime();
    long long elapsed = now - start_ms;
    printf("expire cycle: db %d expired %lld keys in %lld ms\n",
           db->id, expired, elapsed);
    if (elapsed > ACTIVE_EXPIRE_CYCLE_FAST_DURATION) {
        fprintf(stdout, "warning: expire cycle overran its budget (%lld ms)\n",
                elapsed);
        fflush(stdout);
    }
    unsigned long remaining = kvstoreSize(db->expires);
    printf("expire cycle: %lu keys with TTL still tracked\n", remaining);
    updateExpiredStalePerc(expired + remaining, expired);
}
