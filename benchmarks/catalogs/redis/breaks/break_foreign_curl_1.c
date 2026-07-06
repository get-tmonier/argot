#include "server.h"

/* Decoy: idiomatic client reply-buffer accounting in the style of
 * networking.c — NOT part of the break hunk. */
static void trackReplyBytes(client *c, size_t len) {
    c->net_output_bytes += len;
    server.stat_net_output_bytes += len;
}

// Break: libcurl POSTing a webhook to an external HTTP endpoint on a client
// Break: event (curl_easy_init/curl_easy_setopt/curl_easy_perform/
// Break: curl_easy_cleanup/curl_slist_append); libcurl is absent from the
// Break: repo at the pinned SHA (curl_easy_init/curl_easy_setopt/
// Break: curl_easy_perform/curl_easy_cleanup/curl_slist_append = 0 hits
// Break: tree-wide; <curl/curl.h> = 0 hits) — redis speaks only its own RESP
// Break: protocol over conn.c sockets, never a foreign HTTP client.
#include <curl/curl.h>

void postClientWebhook(const char *url, robj *payload) {
    CURL *curl = curl_easy_init();
    if (curl == NULL) return;
    struct curl_slist *headers = NULL;
    headers = curl_slist_append(headers, "Content-Type: application/json");
    curl_easy_setopt(curl, CURLOPT_URL, url);
    curl_easy_setopt(curl, CURLOPT_POSTFIELDS, (char *)payload->ptr);
    curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);
    curl_easy_perform(curl);
    curl_slist_free_all(headers);
    curl_easy_cleanup(curl);
}
