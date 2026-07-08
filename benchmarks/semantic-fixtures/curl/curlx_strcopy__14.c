# ID: lib/curlx/strcopy.c:38
/* Copy src (slen bytes) into dest when it fits, always terminating dest. */
static void bounded_strcopy(char *dest, size_t dsize,
                            const char *src, size_t slen)
{
  DEBUGASSERT(slen < dsize);
  if(slen < dsize) {
    memcpy(dest, src, slen);
    dest[slen] = 0;
  }
  else if(dsize)
    dest[0] = 0;
}
