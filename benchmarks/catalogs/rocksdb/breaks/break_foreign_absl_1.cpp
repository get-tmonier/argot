// Break fixture -- not for compilation into the build.
#include "db/table_cache.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- builds the cache key via Slice.
Slice TableCacheKeyForFile(uint64_t file_number, std::string* scratch) {
  scratch->clear();
  PutVarint64(scratch, file_number);
  return Slice(*scratch);
}

// Break: abseil dependency for an in-process file-number lookup index.
// Break: Zero `#include <absl/` / `absl::flat_hash_map` / `absl::StrCat`
// Break: sites in the corpus at the pinned SHA; rocksdb keys its caches
// Break: through Cache::Lookup/Insert over a Slice key, and reaches for
// Break: std::unordered_map (208 sites in db/) when it needs a plain
// Break: in-process map, never abseil containers or string helpers.
#include <absl/container/flat_hash_map.h>
#include <absl/strings/str_cat.h>

absl::flat_hash_map<uint64_t, std::string> BuildFileNumberDebugIndex(
    const std::vector<uint64_t>& file_numbers) {
  absl::flat_hash_map<uint64_t, std::string> index;
  for (uint64_t file_number : file_numbers) {
    index[file_number] = absl::StrCat("file-", file_number);
  }
  return index;
}

}  // namespace ROCKSDB_NAMESPACE
