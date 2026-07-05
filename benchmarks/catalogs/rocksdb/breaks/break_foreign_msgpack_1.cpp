// Break fixture -- not for compilation into the build.
#include "rocksdb/write_batch.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- appends a length-prefixed field.
void AppendBatchSize(std::string* dst, const WriteBatch& batch) {
  PutVarint64(dst, batch.GetDataSize());
}

// Break: msgpack-c dependency to serialize write-batch stats. Zero
// Break: `#include <msgpack` and zero `msgpack::` sites in the corpus at the
// Break: pinned SHA (git grep); rocksdb serializes with its own coding.h
// Break: (EncodeFixed*/PutVarint*/PutLengthPrefixedSlice), never msgpack.
#include <msgpack.hpp>

std::string PackWriteBatchStats(const WriteBatch& batch) {
  msgpack::sbuffer sbuf;
  msgpack::packer<msgpack::sbuffer> packer(&sbuf);
  packer.pack_map(2);
  packer.pack(std::string("count"));
  packer.pack(static_cast<uint64_t>(batch.Count()));
  packer.pack(std::string("data_size"));
  packer.pack(static_cast<uint64_t>(batch.GetDataSize()));
  return std::string(sbuf.data(), sbuf.size());
}

}  // namespace ROCKSDB_NAMESPACE
