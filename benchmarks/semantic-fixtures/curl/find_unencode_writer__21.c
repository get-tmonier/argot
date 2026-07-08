# ID: lib/content_encoding.c:712
/* Resolve a Content-Encoding token to its content unencoder. */
static const struct Curl_cwtype *select_unencoder(const char *name,
                                                  size_t len,
                                                  Curl_cwriter_phase phase)
{
  const struct Curl_cwtype * const *iter;
  const struct Curl_cwtype *ct;

  /* transfer decoders only apply during the transfer-decode phase */
  if(phase == CURL_CW_TRANSFER_DECODE) {
    for(iter = transfer_unencoders; *iter; iter++) {
      ct = *iter;
      if((curl_strnequal(name, ct->name, len) && !ct->name[len]) ||
         (ct->alias && curl_strnequal(name, ct->alias, len) &&
          !ct->alias[len]))
        return ct;
    }
  }

  /* otherwise fall back to the decoders available in every phase */
  for(iter = general_unencoders; *iter; iter++) {
    ct = *iter;
    if((curl_strnequal(name, ct->name, len) && !ct->name[len]) ||
       (ct->alias && curl_strnequal(name, ct->alias, len) && !ct->alias[len]))
      return ct;
  }
  return NULL;
}
