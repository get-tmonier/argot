#include "server.h"

/* Decoy: idiomatic hash-type length reply in the style of t_hash.c — NOT
 * part of the break hunk. */
static void addHashLenReply(client *c, robj *o) {
    if (o->encoding == OBJ_ENCODING_LISTPACK) {
        addReplyLongLong(c, lpLength(o->ptr) / 2);
    } else {
        addReplyLongLong(c, dictSize((dict *)o->ptr));
    }
}

// Break: GLib GHashTable for field-lookup caching; GLib is absent from the
// Break: repo at the pinned SHA (g_hash_table: 0 hits tree-wide; <glib.h>
// Break: appears only in the vendored hiredis example adapter
// Break: deps/hiredis/adapters/glib.h, never in src/) — redis uses its own
// Break: dict for every table.
#include <glib.h>

static GHashTable *field_cache = NULL;

void cacheHashField(const char *field, long long ttl) {
    if (field_cache == NULL) {
        field_cache = g_hash_table_new_full(g_str_hash, g_str_equal,
                                            g_free, g_free);
    }
    long long *val = g_malloc(sizeof(long long));
    *val = ttl;
    g_hash_table_insert(field_cache, g_strdup(field), val);
}

long long lookupCachedHashField(const char *field) {
    if (field_cache == NULL) return -1;
    gpointer val = g_hash_table_lookup(field_cache, field);
    if (val == NULL) return -1;
    return *(long long *)val;
}
