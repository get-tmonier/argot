#include "curl_setup.h"

/* Decoy: idiomatic resolve-permission check in the style of hostip.c — NOT
 * part of the break hunk. */
static bool host_allows_resolve(struct Curl_easy *data, int port)
{
  return data->set.allowed_protocols && port > 0;
}

// Break: libpq PostgreSQL client writing a DNS-resolution audit row; libpq is
// Break: absent from the repo at the pinned SHA (libpq-fe.h, PQconnectdb,
// Break: PQstatus, PQexec, PQresultStatus, PQclear, PQfinish = 0 hits
// Break: tree-wide) — curl surfaces resolver activity only through its own
// Break: CURL_TRC/infof tracing and its DNS cache, never a foreign RDBMS
// Break: client.
#include <libpq-fe.h>

CURLcode Curl_resolve_audit_pg(const char *conninfo, const char *host, int port)
{
  PGconn *conn = PQconnectdb(conninfo);
  PGresult *res;
  char sql[256];
  if(PQstatus(conn) != CONNECTION_OK) {
    PQfinish(conn);
    return CURLE_COULDNT_CONNECT;
  }
  msnprintf(sql, sizeof(sql),
            "INSERT INTO resolves(host, port) VALUES('%s', %d)", host, port);
  res = PQexec(conn, sql);
  if(PQresultStatus(res) != PGRES_COMMAND_OK) {
    PQclear(res);
    PQfinish(conn);
    return CURLE_WRITE_ERROR;
  }
  PQclear(res);
  PQfinish(conn);
  return CURLE_OK;
}
