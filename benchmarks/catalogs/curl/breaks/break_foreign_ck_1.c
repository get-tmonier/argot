#include "curl_setup.h"

/* Decoy: idiomatic bundle-size check in the style of conncache.c — NOT part
 * of the break hunk. */
static bool cpool_bundle_full(struct cpool_bundle *bundle, size_t max)
{
  return Curl_llist_count(&bundle->conns) >= max;
}

// Break: Concurrency Kit lock-free SPSC ring handing idle connections between
// Break: threads; Concurrency Kit is absent from the repo at the pinned SHA
// Break: (<ck_ring.h>, ck_ring_init, ck_ring_enqueue_spsc,
// Break: ck_ring_dequeue_spsc, ck_pr_load_ptr = 0 hits tree-wide) — curl
// Break: guards its connection cache with the multi handle's own locking and
// Break: linked lists, never a foreign lock-free concurrency library.
#include <ck_ring.h>

static ck_ring_t idle_ring;
static ck_ring_buffer_t idle_buf[256];

void Curl_cpool_ring_push(struct connectdata *conn)
{
  ck_ring_init(&idle_ring, 256);
  ck_ring_enqueue_spsc(&idle_ring, idle_buf, conn);
}

struct connectdata *Curl_cpool_ring_pop(void)
{
  void *conn = NULL;
  if(!ck_ring_dequeue_spsc(&idle_ring, idle_buf, &conn))
    return NULL;
  return ck_pr_load_ptr(&conn);
}
