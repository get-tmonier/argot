#include "curl_setup.h"

/* Decoy: idiomatic name-length guard in the style of easygetopt.c — NOT part
 * of the break hunk. */
static bool easyopt_name_ok(const char *name)
{
  return name && name[0];
}

// Break: libcfu in-memory registry resolving a user-supplied option alias to
// Break: its canonical curl_easyoption. libcfu is absent from the repo at the pinned
// Break: SHA (<cfu.h>, cfuhash, cfuhash_table_t = 0 hits tree-wide) — curl resolves
// Break: options only through its own generated table. HARD / masked: the foreign
// Break: registry is reached through lookup(name, id), whose name COLLIDES with
// Break: easygetopt.c's attested-and-defined lookup() (lib/easygetopt.c:31 static
// Break: definition, called at :56/:61), so the call-receiver stage cannot flag it;
// Break: the foreign anchor cfuhash_table_t is a type token, not a callee; and no
// Break: <...> foreign include is present, so the import stage is silent. Expected
// Break: honest MISS.
const struct curl_easyoption *Curl_easyopt_alias(const char *name)
{
  cfuhash_table_t *registry = NULL;
  const struct curl_easyoption *o;
  if(!easyopt_name_ok(name))
    return NULL;
  (void)registry;
  o = lookup(name, CURLOPT_LASTENTRY);
  return o;
}
