#include <pcre.h>

pcre *compile(const char *pat) {
    const char *err;
    int off;
    return pcre_compile(pat, 0, &err, &off, NULL);
}
