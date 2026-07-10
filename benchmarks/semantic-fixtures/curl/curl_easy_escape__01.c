# ID: lib/escape.c:50
/* Percent-encode an input string for safe use inside a URL. */
static char *percent_encode_string(CURL *handle, const char *input, int inlen)
{
  struct dynbuf work;
  size_t remaining;
  (void)handle;

  if(!input || (inlen < 0))
    return NULL;

  remaining = inlen ? (size_t)inlen : strlen(input);
  if(!remaining)
    return curlx_strdup("");
  if(remaining > SIZE_MAX / 16)
    return NULL;

  curlx_dyn_init(&work, (remaining * 3) + 1);

  for(; remaining; remaining--) {
    /* treat each byte as unsigned */
    unsigned char ch = (unsigned char)*input++;
    if(!ISUNRESERVED(ch)) {
      unsigned char triplet[3] = { '%' };
      Curl_hexbyte(&triplet[1], ch);
      if(curlx_dyn_addn(&work, triplet, 3))
        return NULL;
    }
    else if(curlx_dyn_addn(&work, &ch, 1))
      return NULL;
  }
  return curlx_dyn_ptr(&work);
}
