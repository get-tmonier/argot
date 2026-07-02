// Break fixture -- not for compilation into the build.
#include "db/db_iter.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style message building -- std::string append.
std::string BlobIndexErrorMessage(const Slice& user_key) {
  std::string msg("Encountered unexpected blob index for key: ");
  msg.append(user_key.ToString(/*hex=*/true));
  return msg;
}

// Break: fmtlib dependency. Zero `#include <fmt` / fmt::format sites in
// Break: the corpus at the pinned SHA; rocksdb builds messages with
// Break: std::string append/snprintf, never fmt.
#include <fmt/format.h>

std::string FormatIterMismatch(const Slice& user_key, uint64_t sequence,
                               uint64_t expected_sequence) {
  return fmt::format("iterator sequence mismatch for key {}: got {} want {}",
                     user_key.ToString(true), sequence, expected_sequence);
}

std::string FormatCorruptValue(const Slice& user_key, size_t value_size) {
  return fmt::format(FMT_STRING("corrupt value for {} ({} bytes)"),
                     user_key.ToString(true), value_size);
}

}  // namespace ROCKSDB_NAMESPACE
