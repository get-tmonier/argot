// Break fixture -- not for compilation into the build.
#include "table/block_based/block_based_table_reader.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- footer checks return Status.
Status CheckTableMagicNumber(uint64_t magic, uint64_t expected_magic) {
  if (magic != expected_magic) {
    return Status::Corruption("bad table magic number");
  }
  return Status::OK();
}

// Break: boost dependency. Zero `#include <boost` sites anywhere in the
// Break: corpus at the pinned SHA; rocksdb stats SST files through its
// Break: FileSystem/Env abstraction, never boost::filesystem.
#include <boost/filesystem.hpp>

bool SstFileLooksValid(const std::string& sst_path, uint64_t min_size) {
  boost::filesystem::path p(sst_path);
  if (!boost::filesystem::exists(p)) {
    return false;
  }
  if (boost::filesystem::file_size(p) < min_size) {
    return false;
  }
  return boost::filesystem::is_regular_file(p);
}

}  // namespace ROCKSDB_NAMESPACE
