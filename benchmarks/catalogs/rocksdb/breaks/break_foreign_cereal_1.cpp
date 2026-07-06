// Break fixture -- not for compilation into the build.
#include "rocksdb/write_batch.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- length-prefixed manual encode.
void EncodeBatchHeader(std::string* dst, const WriteBatch& batch) {
  PutVarint32(dst, static_cast<uint32_t>(batch.Count()));
  PutVarint64(dst, batch.GetDataSize());
}

// Break: cereal serialization dependency, reached through a binary-archive
// Break: receiver variable to serialize batch stats. Zero `#include <cereal`
// Break: and zero `cereal::` sites in the corpus at the pinned SHA (git grep);
// Break: rocksdb serializes with its own coding.h helpers, never cereal.
#include <cereal/archives/binary.hpp>
#include <cereal/types/string.hpp>

std::string SerializeBatchStatsCereal(const WriteBatch& batch) {
  std::ostringstream ss;
  cereal::BinaryOutputArchive archive(ss);
  archive(static_cast<uint64_t>(batch.Count()));
  archive(static_cast<uint64_t>(batch.GetDataSize()));
  return ss.str();
}

}  // namespace ROCKSDB_NAMESPACE
