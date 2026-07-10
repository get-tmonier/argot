# ID: lib/slist.c:124
/* Free every node and its payload in a curl_slist chain. */
static void slist_release_chain(struct curl_slist *list)
{
  struct curl_slist *node = list;

  while(node) {
    struct curl_slist *following = node->next;
    curlx_safefree(node->data);
    curlx_free(node);
    node = following;
  }
}
