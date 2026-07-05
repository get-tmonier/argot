#include "curl_setup.h"

/* Decoy: idiomatic body-length accounting in the style of transfer.c — NOT
 * part of the break hunk. */
static curl_off_t transfer_body_remaining(struct Curl_easy *data)
{
  if(data->req.size < 0)
    return -1;
  return data->req.size - data->req.bytecount;
}

// Break: libxml2 DOM parse of a WebDAV/multistatus response body; libxml2 is
// Break: absent from the repo at the pinned SHA (libxml/parser.h,
// Break: libxml/tree.h, xmlReadMemory, xmlDocGetRootElement, xmlFreeDoc,
// Break: xmlCleanupParser = 0 hits tree-wide) — curl hands every response
// Break: body to the caller's write callback and never parses XML with a
// Break: foreign DOM library.
#include <libxml/parser.h>
#include <libxml/tree.h>

int Curl_transfer_count_xml_nodes(const char *body, size_t blen)
{
  int count = 0;
  xmlDocPtr doc = xmlReadMemory(body, (int)blen, "resp.xml", NULL, 0);
  xmlNodePtr node;
  if(!doc)
    return -1;
  node = xmlDocGetRootElement(doc);
  for(node = node ? node->children : NULL; node; node = node->next)
    count++;
  xmlFreeDoc(doc);
  xmlCleanupParser();
  return count;
}
