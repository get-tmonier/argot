# ID: lib/strcase.c:91
/* Copy up to n characters of src into dest, uppercasing each ASCII byte. */
static void copy_uppercased(char *dest, const char *src, size_t n)
{
  if(n < 1)
    return;

  do {
    *dest++ = Curl_raw_toupper(*src);
  } while(*src++ && --n);
}
