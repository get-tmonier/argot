// Break fixture -- not for compilation into the build.
#include "db/blob/blob_file_builder.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- checksums a blob via the repo's
// own crc32c utility over a Slice.
uint32_t BlobRecordChecksum(const Slice& blob) {
  return crc32c::Value(blob.data(), blob.size());
}

// Break: Crypto++ dependency to hash a blob, reached through the
// Break: fully-qualified CryptoPP::SHA256 API (no #include in this hunk --
// Break: the callee itself is the foreign reference). Zero `CryptoPP::` sites
// Break: in the corpus at the pinned SHA (git grep); rocksdb checksums with
// Break: its own crc32c/XXH3 utilities, never a foreign crypto library.
std::string HashBlobSha256(const Slice& blob) {
  unsigned char digest[32];
  CryptoPP::SHA256().CalculateDigest(
      digest, reinterpret_cast<const unsigned char*>(blob.data()), blob.size());
  std::string encoded;
  CryptoPP::HexEncoder encoder;
  encoder.Put(digest, sizeof(digest));
  encoder.MessageEnd();
  encoded.resize(64);
  encoder.Get(reinterpret_cast<unsigned char*>(&encoded[0]), encoded.size());
  return encoded;
}

}  // namespace ROCKSDB_NAMESPACE
