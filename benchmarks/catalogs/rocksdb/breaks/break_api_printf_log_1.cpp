// Break fixture -- not for compilation into the build.
#include "db/wal_manager.h"
#include "logging/logging.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- WAL numbers stay in Status land.
Status CheckWalNumber(uint64_t number, uint64_t min_wal_number) {
  if (number < min_wal_number) {
    return Status::InvalidArgument("WAL number below minimum");
  }
  return Status::OK();
}

// Break: printf/stdout logging. rocksdb logs through ROCKS_LOG_INFO/WARN
// Break: with the Logger from db_options_.info_log -- 14 ROCKS_LOG_ sites
// Break: in this host file (db/wal_manager.cc) alone; printf-style stdout
// Break: logging appears only in benchmark tools, never db/ library code.
#include <cstdio>

void LogWalArchiveProgress(uint64_t number, size_t archived, size_t total) {
  printf("archiving WAL %llu (%zu/%zu)\n",
         static_cast<unsigned long long>(number), archived, total);
  if (archived == total) {
    printf("WAL archive pass complete\n");
    fflush(stdout);
  }
}

void WarnWalTtlExpired(uint64_t number, uint64_t ttl_seconds) {
  fprintf(stdout, "warning: WAL %llu exceeded TTL of %llu seconds\n",
          static_cast<unsigned long long>(number),
          static_cast<unsigned long long>(ttl_seconds));
  fflush(stdout);
}

}  // namespace ROCKSDB_NAMESPACE
