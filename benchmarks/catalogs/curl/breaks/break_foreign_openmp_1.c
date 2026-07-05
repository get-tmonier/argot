#include "curl_setup.h"

/* Decoy: idiomatic decoded-size accumulation in the style of
 * content_encoding.c — NOT part of the break hunk. */
static curl_off_t decoded_total(curl_off_t written, size_t nbytes)
{
  return written + (curl_off_t)nbytes;
}

// Break: OpenMP data-parallel decompression of independent content blocks;
// Break: OpenMP is absent from the repo at the pinned SHA (<omp.h>,
// Break: omp_get_thread_num, omp_get_num_threads, omp_set_num_threads, and
// Break: "#pragma omp" = 0 hits tree-wide) — curl streams every content
// Break: decoding through a single sequential writer chain and never
// Break: parallelizes with a foreign OpenMP runtime.
#include <omp.h>

void Curl_decode_blocks_omp(char **blocks, size_t *sizes, int nblocks)
{
  int i;
  omp_set_num_threads(4);
  #pragma omp parallel for
  for(i = 0; i < nblocks; i++) {
    int tid = omp_get_thread_num();
    blocks[i][0] = (char)(sizes[i] & 0xff);
    (void)tid;
  }
}
