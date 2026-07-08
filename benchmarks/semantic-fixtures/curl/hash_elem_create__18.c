# ID: lib/hash.c:100
/* Allocate a hash element and copy the key bytes into its tail storage. */
static struct Curl_hash_element *hash_element_new(const void *key,
                                                  size_t key_len,
                                                  const void *p,
                                                  Curl_hash_elem_dtor dtor)
{
  struct Curl_hash_element *he =
    curlx_malloc(sizeof(struct Curl_hash_element) + key_len);
  if(!he)
    return NULL;

  he->next = NULL;
  /* copy the key into the space allocated after the struct */
  memcpy(he->key, key, key_len);
  he->key_len = key_len;
  he->ptr = CURL_UNCONST(p);
  he->dtor = dtor;
  return he;
}
