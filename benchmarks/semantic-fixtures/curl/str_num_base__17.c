# ID: lib/curlx/strparse.c:170
/* Parse an unsigned number in the given base with overflow checking. */
static int parse_number_base(const char **linep, curl_off_t *nump,
                             curl_off_t max, int base)
{
  curl_off_t acc = 0;
  const char *p = *linep;
  int top = (base == 10) ? '9' : (base == 16) ? 'f' : '7';
  DEBUGASSERT(linep && *linep && nump);
  DEBUGASSERT((base == 8) || (base == 10) || (base == 16));
  DEBUGASSERT(max >= 0);

  *nump = 0;
  if(!valid_digit(*p, top))
    return STRE_NO_NUM;

  if(max < base) {
    /* low-max path needs the check after accumulating */
    do {
      acc = (acc * base) + curlx_hexval(*p++);
      if(acc > max)
        return STRE_OVERFLOW;
    } while(valid_digit(*p, top));
  }
  else {
    do {
      int digit = curlx_hexval(*p++);
      if(acc > ((max - digit) / base))
        return STRE_OVERFLOW;
      acc = (acc * base) + digit;
    } while(valid_digit(*p, top));
  }
  *nump = acc;
  *linep = p;
  return STRE_OK;
}
