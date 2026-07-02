#include "server.h"

/* Decoy: idiomatic sort-pattern helper in the style of sort.c — NOT part
 * of the break hunk. */
static int patternHasHashField(sds pattern, sds *field) {
    char *p = strchr(pattern, '-');
    if (p && p[1] == '>' && p[2] != '\0') {
        *field = sdsnew(p + 2);
        return 1;
    }
    return 0;
}

// Break: fixed char[] buffer built with strcpy/strcat/sprintf for a
// Break: pattern-substituted key; redis builds every dynamic string as an
// Break: sds (sdsnew/sdscatlen/sdscatprintf, 49 src files at the pinned
// Break: SHA) — strcat has 0 call sites in src/*.c.
static const char *substitutePatternKey(const char *pattern, const char *subst) {
    static char keybuf[256];
    char field[128];
    const char *star = strchr(pattern, '*');
    if (star == NULL) return pattern;
    keybuf[0] = '\0';
    strncat(keybuf, pattern, star - pattern);
    strcat(keybuf, subst);
    strcat(keybuf, star + 1);
    const char *arrow = strstr(keybuf, "->");
    if (arrow != NULL) {
        strcpy(field, arrow + 2);
        sprintf(keybuf + (arrow - keybuf), "{%s}", field);
    }
    return keybuf;
}

void sortByExternalWeight(client *c, const char *pattern, sds element) {
    const char *weightkey = substitutePatternKey(pattern, element);
    robj *key = createStringObject(weightkey, strlen(weightkey));
    kvobj *weight = lookupKeyRead(c->db, key);
    if (weight) addReplyBulk(c, weight);
    decrRefCount(key);
}
