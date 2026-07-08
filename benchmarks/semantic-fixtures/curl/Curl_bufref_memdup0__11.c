# ID: lib/bufref.c:120
/* Copy len bytes into a bufref as a null-terminated, owned string. */
static CURLcode bufref_copy_string(struct bufref *br, const void *ptr,
                                   size_t len)
{
  unsigned char *copy = NULL;

  DEBUGASSERT(br);
  DEBUGASSERT(br->signature == SIGNATURE);
  DEBUGASSERT(ptr || !len);
  DEBUGASSERT(len <= CURL_MAX_INPUT_LENGTH);

  if(ptr) {
    copy = curlx_memdup0(ptr, len);
    if(!copy)
      return CURLE_OUT_OF_MEMORY;
  }

  Curl_bufref_set(br, copy, len, curl_free);
  return CURLE_OK;
}
