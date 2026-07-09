// Break fixture -- not for compilation into the build.
#include "db/flush_job.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- sequential checksum fold.
uint64_t AccumulateChecksums(const std::vector<uint64_t>& crcs) {
  uint64_t acc = 0;
  for (uint64_t c : crcs) {
    acc ^= c;
  }
  return acc;
}

// Break: OpenMP runtime for a parallel checksum fold over table blocks. Zero
// Break: `#include <omp.h>` and zero `#pragma omp` / omp_* call sites in the
// Break: corpus at the pinned SHA (git grep); rocksdb parallelises through its
// Break: own port::Thread pools and Env, never the OpenMP runtime.
#include <omp.h>

uint64_t AccumulateChecksumsParallel(const std::vector<uint64_t>& crcs) {
  uint64_t acc = 0;
  omp_set_num_threads(4);
#pragma omp parallel for reduction(^ : acc)
  for (size_t i = 0; i < crcs.size(); ++i) {
    acc ^= crcs[i];
  }
  return acc;
}

}  // namespace ROCKSDB_NAMESPACE
