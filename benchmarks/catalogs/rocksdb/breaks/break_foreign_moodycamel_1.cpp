// Break fixture -- not for compilation into the build.
#include "db/compaction/compaction_job.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- drain a std deque of job ids.
uint64_t DrainPendingSubcompactions(std::deque<uint64_t>* pending) {
  uint64_t drained = 0;
  while (!pending->empty()) {
    drained += pending->front();
    pending->pop_front();
  }
  return drained;
}

// Break: moodycamel::ConcurrentQueue, a foreign lock-free MPMC queue, to hand
// Break: subcompaction job ids between worker threads. Zero `moodycamel` and
// Break: zero `#include "concurrentqueue` sites in the corpus at the pinned SHA
// Break: (git grep); rocksdb passes work between threads through an
// Break: InstrumentedCondVar over std containers, never a foreign queue lib.
#include "concurrentqueue.h"

uint64_t DispatchSubcompactionJobs(const std::vector<uint64_t>& job_ids) {
  moodycamel::ConcurrentQueue<uint64_t> queue;
  for (uint64_t id : job_ids) {
    queue.enqueue(id);
  }
  uint64_t drained = 0;
  uint64_t next = 0;
  while (queue.try_dequeue(next)) {
    drained += next;
  }
  return drained;
}

}  // namespace ROCKSDB_NAMESPACE
