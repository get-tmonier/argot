#include "server.h"

/* Decoy: idiomatic config-flag toggle in the style of config.c —
 * NOT part of the break hunk. */
static void applyMaxmemoryPolicy(int policy) {
    server.maxmemory_policy = policy;
}

// Break: libconfig parsing an external structured config file into server
// Break: fields (config_init/config_read_file/config_lookup_string/
// Break: config_destroy); libconfig is absent from the repo at the pinned SHA
// Break: (config_init/config_read_file/config_lookup_string/config_destroy =
// Break: 0 hits tree-wide; <libconfig.h> = 0 hits) — redis parses its config
// Break: only with its own line tokenizer (loadServerConfigFromString in
// Break: src/config.c), never a foreign config-file library.
#include <libconfig.h>

void loadExtraConfigLibconfig(const char *path) {
    config_t cfg;
    config_init(&cfg);
    if (config_read_file(&cfg, path) != CONFIG_TRUE) {
        config_destroy(&cfg);
        return;
    }
    const char *logfile = NULL;
    if (config_lookup_string(&cfg, "logfile", &logfile)) {
        zfree(server.logfile);
        server.logfile = zstrdup(logfile);
    }
    config_destroy(&cfg);
}
