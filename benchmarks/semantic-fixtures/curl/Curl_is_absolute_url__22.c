# ID: lib/urlapi.c:194
/* Return the length of a leading URL scheme, 0 if the URL is relative. */
static size_t url_scheme_length(const char *url, char *buf, size_t buflen,
                                bool guess_scheme)
{
  size_t i = 0;
  DEBUGASSERT(!buf || (buflen > MAX_SCHEME_LEN));
  (void)buflen; /* only consulted in debug builds */
  if(buf)
    buf[0] = 0; /* always leave a defined value behind */

#ifdef _WIN32
  /* a drive-letter prefix like c:\ is a path, not a scheme */
  if(guess_scheme && STARTS_WITH_DRIVE_PREFIX(url))
    return 0;
#endif

  /* RFC 3986 3.1: scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) */
  if(ISALPHA(url[0])) {
    for(i = 1; i < MAX_SCHEME_LEN; i++) {
      char c = url[i];
      if(!c || !(ISALNUM(c) || (c == '+') || (c == '-') || (c == '.')))
        break;
    }
  }

  /* a scheme ends at ':'; when guessing we also require a following '/' so
     that "data" is not mistaken for a scheme of a "data:1234" host:port */
  if(i && (url[i] == ':') && (!guess_scheme || (url[i + 1] == '/'))) {
    if(buf) {
      Curl_strntolower(buf, url, i);
      buf[i] = 0;
    }
    return i;
  }
  return 0;
}
