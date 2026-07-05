#include "curl_setup.h"

/* Decoy: idiomatic single-header append in the style of http.c — NOT part
 * of the break hunk. */
static CURLcode add_one_request_header(struct Curl_easy *data,
                                       struct dynbuf *r,
                                       const char *name, const char *value)
{
  (void)data;
  return curlx_dyn_addf(r, "%s: %s\r\n", name, value);
}

// Break: jansson JSON encoding of the outgoing request-header set before send;
// Break: jansson is absent from the repo at the pinned SHA (<jansson.h>,
// Break: json_object, json_object_set_new, json_string, json_array,
// Break: json_dumps, json_decref = 0 hits tree-wide across *.c/*.h) — curl
// Break: builds every request-header line into a dynbuf via curlx_dyn_addf,
// Break: never through a foreign JSON serializer.
#include <jansson.h>

char *Curl_http_headers_to_json(struct Curl_easy *data, const char *ua)
{
  json_t *root = json_object();
  json_t *hdrs = json_array();
  char *encoded;
  (void)data;
  json_array_append_new(hdrs, json_string(ua));
  json_object_set_new(root, "user_agent", json_string(ua));
  json_object_set_new(root, "headers", hdrs);
  encoded = json_dumps(root, JSON_COMPACT);
  json_decref(root);
  return encoded;
}
