# ID: lib/curlx/strparse.c:107
/* Parse a double-quoted token, honoring backslash escapes. */
static int parse_quoted_token(const char **linep, struct Curl_str *out,
                              const size_t max)
{
  const char *cursor = *linep;
  size_t count = 0;
  DEBUGASSERT(linep && *linep && out && max);

  curlx_str_init(out);
  if(*cursor != '\"')
    return STRE_BEGQUOTE;
  cursor++;
  while(*cursor && (*cursor != '\"')) {
    if((*cursor == '\\') && cursor[1]) {
      cursor++;
      if(++count > max)
        return STRE_BIG;
    }
    cursor++;
    if(++count > max)
      return STRE_BIG;
  }
  if(*cursor != '\"')
    return STRE_ENDQUOTE;
  out->str = (*linep) + 1;
  out->len = count;
  *linep = cursor + 1;
  return STRE_OK;
}
