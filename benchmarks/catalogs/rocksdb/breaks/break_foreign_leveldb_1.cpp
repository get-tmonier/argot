// Break fixture -- not for compilation into the build.
#include "db/db_impl/db_impl.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style read helper -- Status-returning single get.
Status GetSingleKey(DB* db, const ReadOptions& options, const Slice& key,
                    std::string* value) {
  return db->Get(options, db->DefaultColumnFamily(), key, value);
}

// Break: LevelDB dependency (a foreign embedded KV store) reached through a
// Break: receiver parameter whose leaf method `.Get` collides with rocksdb's
// Break: own attested method vocabulary (`Get` 674 call sites) -- the sharpest
// Break: possible collision, since LevelDB is rocksdb's own ancestor API. The
// Break: foreign types `leveldb::DB` / `leveldb::ReadOptions` appear only as
// Break: parameter types, so no foreign namespace is ever *called*. Zero
// Break: `leveldb::` / `#include <leveldb` sites in the corpus at the pinned
// Break: SHA (git grep); rocksdb reads through its own DB, never a second
// Break: embedded store. HARD: genuinely may miss.
void MirrorLookupInLevelDb(leveldb::DB* mirror, const leveldb::ReadOptions& ropts,
                           const std::string& key, std::string* value) {
  mirror->Get(ropts, key, value);
}

}  // namespace ROCKSDB_NAMESPACE
