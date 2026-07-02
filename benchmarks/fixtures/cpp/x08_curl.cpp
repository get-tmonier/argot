#include <curl/curl.h>

void fetch(const char *url) {
    CURL *c = curl_easy_init();
    curl_easy_setopt(c, CURLOPT_URL, url);
    curl_easy_perform(c);
}
