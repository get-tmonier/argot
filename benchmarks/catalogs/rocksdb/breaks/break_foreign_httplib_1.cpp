// Break fixture -- not for compilation into the build.
#include "table/block_based/block_based_table_reader.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- footer magic-number check.
Status VerifyTableMagic(uint64_t magic, uint64_t expected_magic) {
  if (magic != expected_magic) {
    return Status::Corruption("bad table magic number");
  }
  return Status::OK();
}

// Break: cpp-httplib dependency (a foreign HTTP client) reached through a
// Break: receiver parameter whose leaf method `.Get` collides with rocksdb's
// Break: own attested method vocabulary (`Get` 674 call sites), masking the
// Break: foreign API from callee resolution. The foreign type `httplib::Client`
// Break: appears only as a parameter type, so no foreign namespace is ever
// Break: *called*. Zero `httplib::` / `#include <httplib` sites in the corpus
// Break: at the pinned SHA (git grep); rocksdb reads blocks through its own
// Break: FileSystem/RandomAccessFile abstraction, never HTTP. HARD: may miss.
Status FetchRemoteBlock(httplib::Client& client, const std::string& path,
                        std::string* out) {
  auto res = client.Get(path.c_str());
  if (!res) {
    return Status::IOError("remote block fetch failed");
  }
  *out = res->body;
  return Status::OK();
}

}  // namespace ROCKSDB_NAMESPACE
