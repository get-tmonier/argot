// Break fixture -- not for compilation into the build.
#include "db/flush_job.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- sequential drain into a vector.
void CollectFileNumbers(const std::vector<uint64_t>& pending,
                        std::vector<uint64_t>* out) {
  for (uint64_t fn : pending) {
    out->push_back(fn);
  }
}

// Break: boost::lockfree (a foreign lock-free concurrency queue) reached
// Break: through a receiver parameter whose leaf methods `.push`/`.pop`
// Break: collide with rocksdb's own attested vocabulary (`push` 45, `pop` 49
// Break: call sites), masking the foreign concurrency primitive from callee
// Break: resolution. The foreign type `boost::lockfree::queue` appears only as
// Break: a parameter type, so no foreign namespace is ever *called*. Zero
// Break: `boost::lockfree` / `#include <boost/lockfree` sites in the corpus at
// Break: the pinned SHA (git grep); rocksdb hands work between threads through
// Break: InstrumentedCondVar over std containers, never a lock-free queue lib.
// Break: HARD: genuinely may miss.
void DrainFlushQueue(boost::lockfree::queue<uint64_t>& pending,
                     const std::vector<uint64_t>& file_numbers) {
  for (uint64_t fn : file_numbers) {
    pending.push(fn);
  }
  uint64_t drained = 0;
  while (pending.pop(drained)) {
    (void)drained;
  }
}

}  // namespace ROCKSDB_NAMESPACE
