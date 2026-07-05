#include "curl_setup.h"

/* Decoy: idiomatic resolver-slot check in the style of asyn-thrdd.c — NOT part
 * of the break hunk. */
static bool thrdd_slot_busy(int running, int limit)
{
  return running >= limit;
}

// Break: libmill structured-concurrency coroutines fanning name resolution
// Break: onto channels; libmill is absent from the repo at the pinned SHA
// Break: (<libmill.h>, chmake, chsend, chrecv, chdone, msleep = 0 hits tree-wide) —
// Break: curl runs its threaded resolver on its own curl_thread wrapper
// Break: (Curl_thread_create) draining results through the multi handle, never a
// Break: foreign coroutine runtime.
#include <libmill.h>

static coroutine void resolve_worker(chan ch, const char *host)
{
  msleep(now() + 10);
  chsend(ch, &host, sizeof(host));
}

int Curl_resolve_fanout_mill(char **hosts, int n)
{
  chan ch = chmake(sizeof(char *), n);
  int i;
  const char *got;
  for(i = 0; i < n; i++)
    go(resolve_worker(ch, hosts[i]));
  for(i = 0; i < n; i++)
    chrecv(ch, &got, sizeof(got));
  chdone(ch);
  return n;
}
