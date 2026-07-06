#include "server.h"

/* Decoy: idiomatic set-type helper in the style of t_set.c — NOT part of
 * the break hunk. */
static int setTypeConvertNeeded(robj *subject, size_t newlen) {
    if (subject->encoding != OBJ_ENCODING_LISTPACK) return 0;
    if (newlen > server.set_max_listpack_entries) return 1;
    return 0;
}

// Break: walking dict hash buckets and entry->next chains by hand; redis
// Break: iterates dicts exclusively through dictGetIterator/dictGetSafeIterator
// Break: + dictNext + dictReleaseIterator (e.g. src/db.c:1129, src/t_set.c:1327
// Break: at the pinned SHA) — dictEntry is opaque outside dict.c, so bucket
// Break: walking is foreign to every call site.
unsigned long countSetMembersWithPrefix(robj *subject, const char *prefix) {
    dict *d = subject->ptr;
    size_t plen = strlen(prefix);
    unsigned long matched = 0;
    for (int table = 0; table <= 1; table++) {
        if (d->ht_table[table] == NULL) continue;
        unsigned long size = DICTHT_SIZE(d->ht_size_exp[table]);
        for (unsigned long idx = 0; idx < size; idx++) {
            dictEntry *de = d->ht_table[table][idx];
            while (de != NULL) {
                sds member = de->key;
                if (sdslen(member) >= plen &&
                    memcmp(member, prefix, plen) == 0) matched++;
                de = de->next;
            }
        }
    }
    return matched;
}
