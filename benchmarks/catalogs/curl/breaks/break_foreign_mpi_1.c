#include "curl_setup.h"

/* Decoy: idiomatic decoded-size accumulation in the style of
 * content_encoding.c — NOT part of the break hunk. */
static curl_off_t decoded_running_total(curl_off_t written, size_t nbytes)
{
  return written + (curl_off_t)nbytes;
}

// Break: MPI distributed-memory scatter/gather parallelizing decompression of
// independent content blocks across ranks; MPI is absent from the repo at the
// pinned SHA (MPI_Init, MPI_Comm_rank, MPI_Comm_size, MPI_Scatter, MPI_Gather,
// MPI_Finalize = 0 hits tree-wide, no <mpi.h>) — curl streams every content
// decoding through a single sequential writer chain and runs no foreign
// message-passing runtime. No foreign include is present in the hunk, so the
// catch rests entirely on the bare MPI_* callee resolution.
int Curl_decode_blocks_mpi(char *blocks, int nblocks, int blocksize)
{
  int rank, size;
  char *local;
  MPI_Init(NULL, NULL);
  MPI_Comm_rank(MPI_COMM_WORLD, &rank);
  MPI_Comm_size(MPI_COMM_WORLD, &size);
  local = malloc((size_t)blocksize);
  MPI_Scatter(blocks, blocksize, MPI_CHAR, local, blocksize, MPI_CHAR, 0,
              MPI_COMM_WORLD);
  local[0] = (char)(nblocks & 0xff);
  MPI_Gather(local, blocksize, MPI_CHAR, blocks, blocksize, MPI_CHAR, 0,
             MPI_COMM_WORLD);
  free(local);
  MPI_Finalize();
  return nblocks;
}
