// Break fixture -- not for compilation into the build.
#include "db/wal_manager.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- plain field comparison.
bool WalExceedsRetentionSize(uint64_t size_bytes, uint64_t max_bytes) {
  return size_bytes > max_bytes;
}

// Break: glog (Google logging) dependency for a WAL retention warning. Zero
// Break: `#include <glog/` and zero `google::` sites anywhere in *.cc/*.h at
// Break: the pinned SHA (git grep); rocksdb logs exclusively through
// Break: ROCKS_LOG_* macros over db_options_.info_log (491 sites in db/),
// Break: never a standalone logging library.
#include <glog/logging.h>

void WarnIfWalOversized(uint64_t log_number, uint64_t size_bytes,
                        uint64_t max_bytes) {
  google::InitGoogleLogging("rocksdb");
  if (size_bytes > max_bytes) {
    LOG(WARNING) << "wal " << log_number << " size " << size_bytes
                 << " exceeds retention " << max_bytes;
  }
}

}  // namespace ROCKSDB_NAMESPACE
