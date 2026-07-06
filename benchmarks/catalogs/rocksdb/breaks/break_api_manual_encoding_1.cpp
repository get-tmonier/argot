// Break fixture -- not for compilation into the build.
#include "db/log_writer.h"
#include "rocksdb/status.h"
#include "util/coding.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- record type checks return Status.
Status CheckRecordType(unsigned int type, unsigned int max_record_type) {
  if (type > max_record_type) {
    return Status::Corruption("unknown record type in log");
  }
  return Status::OK();
}

// Break: hand-rolled little-endian and varint encoding, duplicating the
// Break: util/coding.h utilities. rocksdb standardizes on EncodeFixed32/64
// Break: and PutVarint32 (EncodeFixed32 is used in this very host file,
// Break: db/log_writer.cc:337,347; 37 files use EncodeFixed32/64).
#include <cstring>

void PackLengthPrefix(char* dst, uint32_t value) {
  dst[0] = static_cast<char>(value & 0xff);
  dst[1] = static_cast<char>((value >> 8) & 0xff);
  dst[2] = static_cast<char>((value >> 16) & 0xff);
  dst[3] = static_cast<char>((value >> 24) & 0xff);
}

size_t PackVarLength(std::string* dst, uint32_t value) {
  size_t written = 0;
  while (value >= 0x80) {
    dst->push_back(static_cast<char>((value & 0x7f) | 0x80));
    value >>= 7;
    ++written;
  }
  dst->push_back(static_cast<char>(value));
  return written + 1;
}

uint64_t UnpackFixed64Manual(const char* src) {
  uint64_t result = 0;
  memcpy(&result, src, sizeof(result));
  return result;
}

}  // namespace ROCKSDB_NAMESPACE
