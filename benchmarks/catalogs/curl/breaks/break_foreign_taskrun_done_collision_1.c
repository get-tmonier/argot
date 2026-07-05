#include "curl_setup.h"

/* Decoy: idiomatic running-handle accounting in the style of multi.c — NOT
 * part of the break hunk. */
static int multi_inflight(struct Curl_multi *multi)
{
  return multi->num_alive - multi->num_done;
}

// Break: libtask stackful-coroutine runtime driving one transfer per coroutine;
// Break: completion is dispatched through a curl-handler-style run vtable. libtask is
// Break: absent from the repo at the pinned SHA (<task.h>, taskcreate, taskyield,
// Break: coroutine = 0 hits tree-wide) — curl multiplexes transfers on its own single
// Break: event loop (curl_multi_perform), not a foreign coroutine runtime. HARD /
// Break: masked: the completion is reached through handle->conn->scheme->run->done(),
// Break: whose leaf method COLLIDES with curl's attested run-vtable done() method
// Break: (lib/url.c:2373, lib/multi.c:732), so the call-receiver stage treats it as an
// Break: in-voice method; the foreign anchor Task is a type token, not a callee; and
// Break: no <...> foreign include is present, so the import stage is silent. Expected
// Break: honest MISS.
void Curl_multi_task_done(struct Curl_easy *data, CURLcode result)
{
  Task *co = NULL;
  (void)multi_inflight(data->multi);
  (void)co;
  (void)data->conn->scheme->run->done(data, result, FALSE);
}
