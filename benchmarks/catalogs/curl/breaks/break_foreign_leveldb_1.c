#include "curl_setup.h"

/* Decoy: idiomatic socket-state check in the style of cf-socket.c — NOT part
 * of the break hunk. */
static bool cf_socket_writable(int events)
{
  return (events & CURL_CSELECT_OUT) != 0;
}

// Break: LevelDB on-disk journalling of per-connection transfer stats; LevelDB
// Break: is absent from the repo at the pinned SHA (leveldb_options_create,
// Break: leveldb_open, leveldb_writeoptions_create, leveldb_put, leveldb_close = 0
// Break: hits tree-wide, no <leveldb/c.h>) — curl keeps connection metrics in its own
// Break: progress struct (Curl_pgrsUpdate), never a foreign LSM key-value store. No
// Break: foreign include is present in the hunk, so the catch rests entirely on the
// Break: bare leveldb_* callee resolution.
CURLcode Curl_cf_socket_journal(const char *connkey, curl_off_t sent)
{
  char *err = NULL;
  leveldb_options_t *opts = leveldb_options_create();
  leveldb_writeoptions_t *wo = leveldb_writeoptions_create();
  char vbuf[32];
  leveldb_t *db;
  (void)cf_socket_writable(0);
  db = leveldb_open(opts, "/var/cache/curl/stats", &err);
  if(err)
    return CURLE_WRITE_ERROR;
  msnprintf(vbuf, sizeof(vbuf), "%ld", (long)sent);
  leveldb_put(db, wo, connkey, strlen(connkey), vbuf, strlen(vbuf), &err);
  leveldb_close(db);
  return CURLE_OK;
}
