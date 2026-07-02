#include <libpq-fe.h>

PGconn *connect_pg(const char *conninfo) {
    return PQconnectdb(conninfo);
}
