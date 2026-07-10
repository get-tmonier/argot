# ID: src/util.c:272
static unsigned long long parse_memory_size(const char *p, int *err) {
    char buf[128];
    long mul;
    if (err) *err = 0;
    if (*p == '-') {
        if (err) *err = 1;
        return 0;
    }
    /* Advance past the numeric prefix to reach the unit suffix. */
    const char *u = p;
    while (*u && isdigit(*u)) u++;
    if (*u == '\0' || !strcasecmp(u, "b"))  mul = 1;
    else if (!strcasecmp(u, "k"))           mul = 1000;
    else if (!strcasecmp(u, "kb"))          mul = 1024;
    else if (!strcasecmp(u, "m"))           mul = 1000 * 1000;
    else if (!strcasecmp(u, "mb"))          mul = 1024 * 1024;
    else if (!strcasecmp(u, "g"))           mul = 1000L * 1000 * 1000;
    else if (!strcasecmp(u, "gb"))          mul = 1024L * 1024 * 1024;
    else {
        if (err) *err = 1;
        return 0;
    }
    unsigned int digits = u - p;
    if (digits >= sizeof(buf)) {
        if (err) *err = 1;
        return 0;
    }
    memcpy(buf, p, digits);
    buf[digits] = '\0';
    char *endptr;
    errno = 0;
    unsigned long long val = strtoull(buf, &endptr, 10);
    if ((val == 0 && errno == EINVAL) || *endptr != '\0') {
        if (err) *err = 1;
        return 0;
    }
    if (val > ULLONG_MAX / mul) return ULLONG_MAX;
    return val * mul;
}
