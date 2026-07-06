// Break fixture -- not for compilation into the build.
#include "db/compaction/compaction_job.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- bounded subcompaction count.
uint64_t ClampSubcompactions(uint64_t requested, uint64_t max_allowed) {
  if (requested == 0) {
    return 1;
  }
  return requested > max_allowed ? max_allowed : requested;
}

// Break: detached std::thread fire-and-forget with sleep_for polling.
// Break: rocksdb runs subcompactions in std::vector<port::Thread> and joins
// Break: every worker (db/compaction/compaction_job.cc:732,895); there are
// Break: zero `.detach()` call sites in db/*.cc at the pinned SHA.
#include <atomic>
#include <chrono>
#include <thread>

static std::atomic<bool> compaction_kicked{false};

void KickCompactionDetached() {
  std::thread([] {
    std::this_thread::sleep_for(std::chrono::milliseconds(5));
    compaction_kicked.store(true);
  }).detach();
}

void WaitForDetachedCompaction() {
  KickCompactionDetached();
  while (!compaction_kicked.load()) {
    std::this_thread::sleep_for(std::chrono::milliseconds(1));
  }
  std::thread cleanup([] { compaction_kicked.store(false); });
  cleanup.detach();
}

}  // namespace ROCKSDB_NAMESPACE
