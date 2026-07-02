// Break fixture -- not for compilation into the build.
#include "db/blob/blob_file_builder.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- sizes flow through Status paths.
Status CheckBlobRecordSize(uint64_t key_size, uint64_t value_size) {
  if (key_size == 0) {
    return Status::Corruption("blob record has empty key");
  }
  if (value_size == 0) {
    return Status::Corruption("blob record has empty value");
  }
  return Status::OK();
}

// Break: direct FILE* stdio for file writes. rocksdb routes all file IO
// Break: through the FileSystem/Env abstraction -- this host file itself
// Break: writes blobs via WritableFileWriter/FSWritableFile; there are zero
// Break: fopen() call sites in db/*.cc library code at the pinned SHA.
#include <cstdio>

bool WriteBlobIndexToDisk(const std::string& path, const Slice& key,
                          const Slice& blob) {
  FILE* fp = fopen(path.c_str(), "ab");
  if (fp == nullptr) {
    return false;
  }
  fwrite(key.data(), 1, key.size(), fp);
  fwrite(blob.data(), 1, blob.size(), fp);
  fflush(fp);
  fclose(fp);
  return true;
}

long ReadBlobFileSize(const std::string& path) {
  FILE* fp = fopen(path.c_str(), "rb");
  if (fp == nullptr) {
    return -1;
  }
  fseek(fp, 0, SEEK_END);
  long size = ftell(fp);
  fclose(fp);
  return size;
}

}  // namespace ROCKSDB_NAMESPACE
