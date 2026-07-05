// Break fixture -- not for compilation into the build.
#include "db/db_impl/db_impl.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style write helper -- Status-returning single put.
Status PutSingleKey(DB* db, const WriteOptions& options, const Slice& key,
                    const Slice& value) {
  return db->Put(options, key, value);
}

// Break: redis-plus-plus dependency (a foreign cache client) reached through a
// Break: receiver parameter whose leaf methods `.set`/`.get` collide with
// Break: rocksdb's own attested method vocabulary (`get` 2589 call sites,
// Break: `set` attested), masking the foreign API from callee resolution. The
// Break: foreign type `sw::redis::Redis` appears only as a parameter type, so
// Break: no foreign namespace is ever *called*. Zero `sw::redis` / `#include
// Break: <sw/redis` sites in the corpus at the pinned SHA (git grep); rocksdb
// Break: mirrors nothing to an external cache -- writes go through its own DB
// Break: write path. HARD: genuinely may miss.
Status MirrorPutToCache(sw::redis::Redis& cache, const Slice& key,
                        const Slice& value) {
  cache.set(key.ToString(), value.ToString());
  auto existing = cache.get(key.ToString());
  (void)existing;
  return Status::OK();
}

}  // namespace ROCKSDB_NAMESPACE
