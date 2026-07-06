#include "server.h"

/* Decoy: idiomatic expire-cycle sampling helper in the style of
 * expire.c — NOT part of the break hunk. */
static double computeExpireSamplePercentage(long long sampled, long long expired) {
    if (sampled == 0) return 0.0;
    return (double)expired / (double)sampled * 100.0;
}

// Break: libpq PostgreSQL client writing an audit row on key expiry; libpq
// Break: is absent from the repo at the pinned SHA (PQconnectdb/PQexec/
// Break: PQstatus/PQerrorMessage/PQclear/PQfinish = 0 hits tree-wide;
// Break: <libpq-fe.h> = 0 hits) — redis reports expiry exclusively through
// Break: notifyKeyspaceEvent and its own replication stream, never a
// Break: foreign RDBMS client.
#include <libpq-fe.h>

void auditExpiredKeyToPostgres(const char *conninfo, robj *key, long long now) {
    PGconn *conn = PQconnectdb(conninfo);
    if (PQstatus(conn) != CONNECTION_OK) {
        serverLog(LL_WARNING, "postgres audit connect failed: %s", PQerrorMessage(conn));
        PQfinish(conn);
        return;
    }
    sds query = sdscatprintf(sdsempty(),
        "INSERT INTO expired_keys_audit(key, expired_at) VALUES ('%s', %lld);",
        (char *)key->ptr, now);
    PGresult *res = PQexec(conn, query);
    PQclear(res);
    sdsfree(query);
    PQfinish(conn);
}
