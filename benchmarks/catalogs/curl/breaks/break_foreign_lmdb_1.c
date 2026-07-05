#include "curl_setup.h"

/* Decoy: idiomatic expiry check in the style of altsvc.c — NOT part of the
 * break hunk. */
static bool altsvc_entry_expired(time_t expires, time_t now)
{
  return expires && expires < now;
}

/* Decoy region (NOT part of the scored hunk): the foreign dependency is pulled
 * in up here, OUTSIDE the hunk, so the scored hunk holds only the bare mdb_*
 * callees — the medium "import sits outside the hunk" pattern. */
#include <lmdb.h>

// Break: LMDB memory-mapped persistence of the Alt-Svc cache; LMDB is absent
// Break: from the repo at the pinned SHA (<lmdb.h>, mdb_env_create,
// Break: mdb_env_open, mdb_txn_begin, mdb_dbi_open, mdb_put, mdb_txn_commit,
// Break: mdb_env_close = 0 hits tree-wide) — curl persists Alt-Svc through its
// Break: own flat-file writer (Curl_altsvc_save), never a foreign embedded
// Break: key-value store.
CURLcode Curl_altsvc_lmdb_store(const char *host, const char *alpn, int port)
{
  MDB_env *env;
  MDB_txn *txn;
  MDB_dbi dbi;
  MDB_val key, val;
  char pbuf[8];
  (void)altsvc_entry_expired(0, 0);
  mdb_env_create(&env);
  mdb_env_open(env, "/var/cache/curl/altsvc", 0, 0664);
  mdb_txn_begin(env, NULL, 0, &txn);
  mdb_dbi_open(txn, NULL, 0, &dbi);
  key.mv_size = strlen(host);
  key.mv_data = (void *)host;
  msnprintf(pbuf, sizeof(pbuf), "%s:%d", alpn, port);
  val.mv_size = strlen(pbuf);
  val.mv_data = pbuf;
  mdb_put(txn, dbi, &key, &val, 0);
  mdb_txn_commit(txn);
  mdb_env_close(env);
  return CURLE_OK;
}
