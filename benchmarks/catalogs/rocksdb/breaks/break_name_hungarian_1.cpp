// Break fixture -- not for compilation into the build.
#include "db/dbformat.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style naming -- PascalCase free function with
// snake_case locals (mirrors AppendUserKeyWithMaxTimestamp in the host).
void AppendPaddedUserKey(std::string* result, const Slice& key,
                         size_t pad_bytes) {
  result->append(key.data(), key.size());
  result->append(pad_bytes, static_cast<char>(0));
}

// Break: Hungarian notation and camelCase locals. rocksdb locals and
// Break: parameters are snake_case (ts_sz, user_key in db/dbformat.cc);
// Break: psz/dw/m_i prefixes and camelCase like userKeyLen appear nowhere
// Break: in the corpus at the pinned SHA.
size_t ComputeKeyLen(const char* pszUserKey, size_t nKeySize,
                     uint32_t dwFlags) {
  size_t userKeyLen = nKeySize;
  if (dwFlags & 0x1) {
    userKeyLen += 8;
  }
  const char* pchCursor = pszUserKey;
  size_t nPadding = 0;
  while (pchCursor != nullptr && nPadding < userKeyLen && *pchCursor == 0) {
    ++nPadding;
    ++pchCursor;
  }
  return userKeyLen - nPadding;
}

class KeyStatsCollector {
 public:
  void RecordKey(size_t nKeySize) {
    m_iTotalKeys += 1;
    m_cbTotalBytes += nKeySize;
  }

 private:
  int m_iTotalKeys = 0;
  size_t m_cbTotalBytes = 0;
};

}  // namespace ROCKSDB_NAMESPACE
