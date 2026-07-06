// Break fixture -- not for compilation into the build.
#include "rocksdb/write_batch.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style check -- plain field comparison.
bool WriteBatchExceedsSizeLimit(const WriteBatch& batch, size_t max_bytes) {
  return batch.GetDataSize() > max_bytes;
}

// Break: spdlog dependency for a size-limit warning. Zero `#include
// Break: <spdlog/spdlog` / `spdlog::logger` / `spdlog::info(` sites in the
// Break: corpus at the pinned SHA; rocksdb logs exclusively through
// Break: ROCKS_LOG_* macros over db_options_.info_log (491 sites in db/),
// Break: never a standalone logging library.
#include <spdlog/spdlog.h>

void WarnIfWriteBatchOversized(const WriteBatch& batch, size_t max_bytes,
                               std::shared_ptr<spdlog::logger> logger) {
  if (batch.GetDataSize() > max_bytes) {
    spdlog::info("write batch of {} bytes exceeds limit {}",
                batch.GetDataSize(), max_bytes);
    logger->warn("oversized write batch: {} > {}", batch.GetDataSize(),
                max_bytes);
  }
}

}  // namespace ROCKSDB_NAMESPACE
