# ID: lib/curl_get_line.c:35
/* Read one complete newline-terminated line from input into buf. */
static CURLcode read_full_line(struct dynbuf *buf, FILE *input, bool *eof)
{
  char chunk[128];

  curlx_dyn_reset(buf);
  for(;;) {
    size_t total;
    const char *line;
    const char *got = fgets(chunk, sizeof(chunk), input);
    if(!got && ferror(input))
      return CURLE_READ_ERROR;

    *eof = feof(input);

    if(got) {
      size_t chunklen = strlen(got);
      if(chunklen) {
        CURLcode result = curlx_dyn_addn(buf, got, chunklen);
        if(result)
          /* line too long or out of memory */
          return result;
      }
    }

    total = curlx_dyn_len(buf);
    line = curlx_dyn_ptr(buf);
    if(total && (line[total - 1] == '\n'))
      return CURLE_OK;
    if(*eof)
      return curlx_dyn_addn(buf, "\n", 1);
  }
}
