#include "curl_setup.h"

/* Decoy: idiomatic TTL check in the style of doh.c — NOT part of the break
 * hunk. */
static bool doh_entry_fresh(time_t inserted, time_t now, int ttl)
{
  return (now - inserted) < ttl;
}

// Break: libmemcached caching of a resolved DoH answer in a remote memcached
// Break: cluster; libmemcached is absent from the repo at the pinned SHA
// Break: (<libmemcached/memcached.h>, memcached_create, memcached_server_add,
// Break: memcached_set, memcached_get, memcached_free = 0 hits tree-wide) — curl
// Break: caches DNS answers in its own in-process hash (Curl_dnscache), never a
// Break: foreign distributed cache client.
#include <libmemcached/memcached.h>

CURLcode Curl_doh_cache_put(const char *host, const char *answer, size_t len)
{
  memcached_st *memc = memcached_create(NULL);
  memcached_return_t rc;
  memcached_server_add(memc, "127.0.0.1", 11211);
  rc = memcached_set(memc, host, strlen(host), answer, len, (time_t)60, 0);
  memcached_free(memc);
  return (rc == MEMCACHED_SUCCESS) ? CURLE_OK : CURLE_WRITE_ERROR;
}
