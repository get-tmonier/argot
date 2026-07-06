#include "curl_setup.h"

/* Decoy: idiomatic default-port lookup in the style of url.c — NOT part of
 * the break hunk. */
static int url_default_port(const char *scheme)
{
  if(curl_strequal(scheme, "https"))
    return 443;
  if(curl_strequal(scheme, "http"))
    return 80;
  return 0;
}

// Break: cJSON serialization of the parsed URL components; cJSON is absent
// Break: from the repo at the pinned SHA (cjson/cJSON.h, cJSON_CreateObject,
// Break: cJSON_AddStringToObject, cJSON_AddNumberToObject,
// Break: cJSON_PrintUnformatted, cJSON_Delete = 0 hits tree-wide) — curl
// Break: exposes URL parts through its own CURLU handle and curl_url_get,
// Break: never a foreign JSON library.
#include <cjson/cJSON.h>

char *Curl_url_to_json(const char *scheme, const char *host, int port)
{
  cJSON *obj = cJSON_CreateObject();
  char *text;
  cJSON_AddStringToObject(obj, "scheme", scheme);
  cJSON_AddStringToObject(obj, "host", host);
  cJSON_AddNumberToObject(obj, "port", port);
  text = cJSON_PrintUnformatted(obj);
  cJSON_Delete(obj);
  return text;
}
