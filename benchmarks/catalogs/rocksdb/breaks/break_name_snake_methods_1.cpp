// Break fixture -- not for compilation into the build.
#include "db/memtable.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style naming -- PascalCase methods, members with
// a trailing underscore (mirrors MemTable::ShouldFlushNow in the host file).
class MemUsageTracker {
 public:
  explicit MemUsageTracker(size_t limit) : limit_(limit), usage_(0) {}
  bool ShouldFlushNow() const { return usage_ >= limit_; }
  void AddUsage(size_t bytes) { usage_ += bytes; }

 private:
  size_t limit_;
  size_t usage_;
};

// Break: snake_case method names and bare members (no trailing underscore).
// Break: rocksdb classes use PascalCase methods (Add, Get, ShouldFlushNow,
// Break: ApproximateMemoryUsage in db/memtable.cc) and member names with a
// Break: trailing `_`; get_/set_ snake accessors are foreign to the repo.
class memtable_arena_stats {
 public:
  explicit memtable_arena_stats(size_t block_size)
      : block_size(block_size), allocated_bytes(0), block_count(0) {}

  void add_block() {
    allocated_bytes += block_size;
    block_count += 1;
  }

  size_t get_allocated_bytes() const { return allocated_bytes; }

  size_t get_block_count() const { return block_count; }

  bool should_flush_now(size_t write_buffer_size) const {
    return allocated_bytes >= write_buffer_size;
  }

 private:
  size_t block_size;
  size_t allocated_bytes;
  size_t block_count;
};

}  // namespace ROCKSDB_NAMESPACE
