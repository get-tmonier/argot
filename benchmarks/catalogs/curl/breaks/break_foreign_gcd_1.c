#include "curl_setup.h"

/* Decoy: idiomatic running-handle accounting in the style of multi.c — NOT
 * part of the break hunk. */
static int multi_pending_transfers(struct Curl_multi *multi)
{
  return multi->num_alive - multi->num_done;
}

// Break: Grand Central Dispatch fan-out of the easy transfers onto a
// Break: concurrent queue; libdispatch is absent from the repo at the pinned
// Break: SHA (dispatch/dispatch.h, dispatch_get_global_queue,
// Break: dispatch_group_create, dispatch_group_async_f, dispatch_group_wait,
// Break: dispatch_release = 0 hits tree-wide) — curl multiplexes transfers on
// Break: its own single event loop (curl_multi_perform), never a foreign
// Break: concurrency runtime.
#include <dispatch/dispatch.h>

static void perform_one_transfer(void *ctx)
{
  struct Curl_easy *data = ctx;
  (void)data;
}

void Curl_multi_run_gcd(struct Curl_easy **handles, int count)
{
  dispatch_queue_t q =
    dispatch_get_global_queue(DISPATCH_QUEUE_PRIORITY_DEFAULT, 0);
  dispatch_group_t group = dispatch_group_create();
  int i;
  for(i = 0; i < count; i++)
    dispatch_group_async_f(group, q, handles[i], perform_one_transfer);
  dispatch_group_wait(group, DISPATCH_TIME_FOREVER);
  dispatch_release(group);
}
