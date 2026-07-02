// Break fixture -- not for compilation into the build.
#include "db/table_cache.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- corruption becomes a Status.
Status CheckTableHandle(Cache::Handle* handle, uint64_t file_number) {
  if (handle == nullptr) {
    return Status::Corruption("table handle missing for file " +
                              std::to_string(file_number));
  }
  return Status::OK();
}

// Break: process-terminating error handling in library paths.
// Break: zero exit()/abort() calls in db/*.cc library code at the pinned
// Break: SHA; rocksdb reports corruption via Status::Corruption returns.
#include <cstdio>
#include <cstdlib>

void EvictStaleTableOrDie(Cache* cache, uint64_t file_number,
                          Cache::Handle* handle) {
  if (cache == nullptr) {
    fprintf(stderr, "fatal: table cache is null\n");
    exit(1);
  }
  if (handle == nullptr) {
    fprintf(stderr, "fatal: no handle for file %llu\n",
            static_cast<unsigned long long>(file_number));
    abort();
  }
  cache->Release(handle);
}

int GuardTableFileNumber(uint64_t file_number) {
  if (file_number == 0) {
    fprintf(stderr, "invalid file number 0, terminating\n");
    exit(EXIT_FAILURE);
  }
  return 0;
}

}  // namespace ROCKSDB_NAMESPACE
