# ID: lib/escape.c:163
/* Unescape a percent-encoded URL string into freshly allocated memory. */
static char *url_decode_alloc(CURL *handle, const char *input, int inlen,
                              int *outlen)
{
  char *decoded = NULL;
  (void)handle;

  if(!input || (inlen < 0))
    return NULL;

  {
    size_t inbytes = (size_t)inlen;
    size_t produced;
    CURLcode rc = Curl_urldecode(input, inbytes, &decoded, &produced,
                                 REJECT_NADA);
    if(rc)
      return NULL;

    if(outlen) {
      if(produced <= (size_t)INT_MAX)
        *outlen = curlx_uztosi(produced);
      else
        /* too large to fit an int, bail out */
        curlx_safefree(decoded);
    }
  }
  return decoded;
}
