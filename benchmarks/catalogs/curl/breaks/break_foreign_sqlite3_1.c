#include "curl_setup.h"

/* Decoy: idiomatic session-cookie expiry test in the style of cookie.c — NOT
 * part of the break hunk. */
static bool cookie_is_expired(struct Cookie *co, time_t now)
{
  return co->expires && co->expires < (curl_off_t)now;
}

// Break: sqlite3 side-table persistence of the cookie jar for durability;
// Break: sqlite3 is absent from the repo at the pinned SHA (<sqlite3.h>,
// Break: sqlite3_open, sqlite3_prepare_v2, sqlite3_bind_text, sqlite3_step,
// Break: sqlite3_finalize, sqlite3_close = 0 hits tree-wide) — curl persists
// Break: cookies exclusively through its own Netscape-format flat file
// Break: (Curl_cookie_output), never a foreign embedded database.
#include <sqlite3.h>

CURLcode Curl_cookie_persist_sqlite(const char *path, struct Cookie *co)
{
  sqlite3 *db = NULL;
  sqlite3_stmt *stmt = NULL;
  if(sqlite3_open(path, &db) != SQLITE_OK)
    return CURLE_WRITE_ERROR;
  sqlite3_prepare_v2(db,
    "INSERT OR REPLACE INTO cookies(name, value) VALUES(?, ?)", -1, &stmt, NULL);
  sqlite3_bind_text(stmt, 1, co->name, -1, NULL);
  sqlite3_bind_text(stmt, 2, co->value, -1, NULL);
  if(sqlite3_step(stmt) != SQLITE_DONE) {
    sqlite3_finalize(stmt);
    sqlite3_close(db);
    return CURLE_WRITE_ERROR;
  }
  sqlite3_finalize(stmt);
  sqlite3_close(db);
  return CURLE_OK;
}
