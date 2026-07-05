// Break fixture -- not for compilation into the build.
#include "db/wal_manager.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- plain suffix check.
bool HasLogSuffix(const std::string& fname) {
  return fname.size() > 4 && fname.compare(fname.size() - 4, 4, ".log") == 0;
}

// Break: Google RE2 regex dependency to validate a WAL filename, reached
// Break: through a compiled RE2 matcher and the fully-qualified RE2::FullMatch
// Break: API. Zero `#include <re2/` and zero `RE2::` call sites in the corpus
// Break: at the pinned SHA (git grep; the token "RE2" matches only
// Break: PPC_FEATURE2 in util/crc32c.cc, not the regex library); rocksdb
// Break: parses filenames with ParseFileName over its own numeric scan.
#include <re2/re2.h>

bool WalFilenameMatchesPattern(const std::string& fname, uint64_t* log_number) {
  RE2 pattern(R"(([0-9]+)\.log)");
  int parsed = 0;
  if (RE2::FullMatch(fname, pattern, &parsed)) {
    *log_number = static_cast<uint64_t>(parsed);
    return true;
  }
  return false;
}

}  // namespace ROCKSDB_NAMESPACE
