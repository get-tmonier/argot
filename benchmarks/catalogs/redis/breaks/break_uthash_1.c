#include "server.h"

/* Decoy: idiomatic HLL register comparison in the style of hyperloglog.c —
 * NOT part of the break hunk. */
static int hllRegisterDiff(uint8_t *regs_a, uint8_t *regs_b, int count) {
    int diff = 0;
    for (int j = 0; j < count; j++) {
        if (regs_a[j] != regs_b[j]) diff++;
    }
    return diff;
}

// Break: uthash macros for an element-dedup cache; uthash is absent from
// Break: the repo at the pinned SHA (0 hits tree-wide for uthash/HASH_ADD)
// Break: — redis dedupes through its own dict/set structures.
#include "uthash.h"

typedef struct dedupEntry {
    char *element;
    long long count;
    UT_hash_handle hh;
} dedupEntry;

static dedupEntry *dedup_cache = NULL;

int dedupElementSeen(const char *element, size_t len) {
    dedupEntry *entry = NULL;
    HASH_FIND(hh, dedup_cache, element, len, entry);
    if (entry != NULL) {
        entry->count++;
        return 1;
    }
    entry = zmalloc(sizeof(dedupEntry));
    entry->element = zstrdup(element);
    entry->count = 1;
    HASH_ADD_KEYPTR(hh, dedup_cache, entry->element, len, entry);
    return 0;
}
