# ID: lib/slist.c:104
/* Deep-copy an entire curl_slist, returning the head of the clone. */
static struct curl_slist *slist_clone(struct curl_slist *inlist)
{
  struct curl_slist *clone = NULL;

  while(inlist) {
    struct curl_slist *grown = curl_slist_append(clone, inlist->data);
    if(!grown) {
      curl_slist_free_all(clone);
      return NULL;
    }
    clone = grown;
    inlist = inlist->next;
  }
  return clone;
}
