# ID: lib/escape.c:201
/* Render binary bytes as a lowercase, null-terminated hex string. */
static void bytes_to_lowercase_hex(const unsigned char *src, size_t len,
                                   unsigned char *out, size_t olen)
{
  DEBUGASSERT(src && len && (olen >= 3));
  if(!src || !len || (olen < 3)) {
    if(olen)
      *out = 0;
    return;
  }
  for(; len && (olen >= 3); len--) {
    out[0] = Curl_ldigits[*src >> 4];
    out[1] = Curl_ldigits[*src & 0x0F];
    src++;
    out += 2;
    olen -= 2;
  }
  *out = 0;
}
