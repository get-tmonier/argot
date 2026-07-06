// Break fixture -- not for compilation into the build.
#include "db/flush_job.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- sequential accumulation.
uint64_t SumEntrySizes(const std::vector<uint64_t>& sizes) {
  uint64_t total = 0;
  for (uint64_t s : sizes) {
    total += s;
  }
  return total;
}

// Break: Intel TBB parallel task runtime for a parallel memtable scan. Zero
// Break: `#include <tbb/` and zero `tbb::parallel_for` sites in the corpus at
// Break: the pinned SHA (git grep); rocksdb parallelises work through its own
// Break: port::Thread pools and Env, never a foreign task runtime.
#include <tbb/parallel_for.h>
#include <tbb/blocked_range.h>

uint64_t SumEntrySizesParallel(const std::vector<uint64_t>& sizes) {
  std::atomic<uint64_t> total{0};
  tbb::parallel_for(tbb::blocked_range<size_t>(0, sizes.size()),
                    [&](const tbb::blocked_range<size_t>& r) {
                      uint64_t local = 0;
                      for (size_t i = r.begin(); i != r.end(); ++i) {
                        local += sizes[i];
                      }
                      total.fetch_add(local);
                    });
  return total.load();
}

}  // namespace ROCKSDB_NAMESPACE
