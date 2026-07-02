// Break fixture -- not for compilation into the build.
#include "db/write_batch_internal.h"
#include "rocksdb/status.h"
#include "util/coding.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- Status return, no exceptions.
Status ValidateBatchCount(const WriteBatch* batch, uint32_t expected) {
  if (batch == nullptr) {
    return Status::InvalidArgument("write batch is null");
  }
  const uint32_t count = WriteBatchInternal::Count(batch);
  if (count != expected) {
    return Status::Corruption("write batch count mismatch");
  }
  return Status::OK();
}

// Break: exception-based error discipline in a Status-return codebase.
// Break: rocksdb returns rocksdb::Status everywhere; `throw` appears once
// Break: repo-wide in db/ (memtable.cc:494) and try/catch never in db/ lib.
#include <stdexcept>

void CheckBatchPrefix(const Slice& contents) {
  if (contents.size() < 12) {
    throw std::runtime_error("write batch header too small: " +
                             std::to_string(contents.size()));
  }
}

uint32_t DecodeBatchCountOrThrow(const Slice& contents) {
  CheckBatchPrefix(contents);
  const char* ptr = contents.data() + 8;
  uint32_t count = DecodeFixed32(ptr);
  if (count == 0) {
    throw std::invalid_argument("empty write batch is not allowed");
  }
  return count;
}

bool TryDecodeBatchCount(const Slice& contents, uint32_t* out) {
  try {
    *out = DecodeBatchCountOrThrow(contents);
    return true;
  } catch (const std::exception& e) {
    return false;
  }
}

}  // namespace ROCKSDB_NAMESPACE
