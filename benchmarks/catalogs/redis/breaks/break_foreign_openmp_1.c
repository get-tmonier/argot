#include "server.h"

/* Decoy: idiomatic debug digest accumulator in the style of debug.c —
 * NOT part of the break hunk. */
static void mixDigestInto(unsigned char *digest, const void *ptr, size_t len) {
    SHA1_CTX ctx;
    SHA1Init(&ctx);
    SHA1Update(&ctx, ptr, len);
    SHA1Final(digest, &ctx);
}

// Break: OpenMP data-parallel checksum of the keyspace across worker threads
// Break: (<omp.h> + #pragma omp parallel for + omp_set_num_threads/
// Break: omp_get_thread_num/omp_get_num_threads); OpenMP is absent from the
// Break: repo at the pinned SHA (omp_set_num_threads/omp_get_thread_num/
// Break: omp_get_num_threads = 0 hits tree-wide; <omp.h> = 0 hits) — redis is
// Break: single-threaded on the serving path and offloads only fixed jobs to
// Break: the bio.c worker pool, never a foreign fork-join parallel runtime.
#include <omp.h>

uint64_t parallelKeyspaceChecksum(robj **objects, int count) {
    uint64_t total = 0;
    omp_set_num_threads(4);
    #pragma omp parallel for reduction(+:total)
    for (int j = 0; j < count; j++) {
        int tid = omp_get_thread_num();
        int nthreads = omp_get_num_threads();
        serverLog(LL_DEBUG, "worker %d/%d hashing", tid, nthreads);
        total += crc64(0, objects[j]->ptr, sdslen(objects[j]->ptr));
    }
    return total;
}
