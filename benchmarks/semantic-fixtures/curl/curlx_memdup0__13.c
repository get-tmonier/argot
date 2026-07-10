# ID: lib/curlx/strdup.c:85
/* Duplicate length bytes from src and append a null terminator. */
static void *memdup_terminated(const char *src, size_t length)
{
  char *buf;

  if(length >= SIZE_MAX)
    return NULL;

  buf = curlx_malloc(length + 1);
  if(!buf)
    return NULL;

  if(length) {
    DEBUGASSERT(src); /* must never be NULL */
    memcpy(buf, src, length);
  }
  buf[length] = 0;
  return buf;
}
