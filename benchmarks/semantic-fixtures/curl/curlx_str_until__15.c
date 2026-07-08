# ID: lib/curlx/strparse.c:49
/* Capture bytes up to the delimiter into out; error if empty or too long. */
static int scan_word_until(const char **linep, struct Curl_str *out,
                           const size_t max, char delim)
{
  const char *cursor;
  size_t count = 0;
  DEBUGASSERT(linep && *linep && out && delim);

  cursor = *linep;
  curlx_str_init(out);
  while(*cursor && (*cursor != delim)) {
    cursor++;
    if(++count > max)
      return STRE_BIG;
  }
  if(!count)
    return STRE_SHORT;
  out->str = *linep;
  out->len = count;
  *linep = cursor; /* first byte past the word */
  return STRE_OK;
}
