// Break fixture -- not for compilation into the build.
#include "db/db_impl/db_impl.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- failure surfaces as a Status.
Status CheckWriteOptionsForDelete(const WriteOptions& write_options) {
  if (write_options.sync && write_options.disableWAL) {
    return Status::InvalidArgument("Sync writes has to enable WAL.");
  }
  return Status::OK();
}

// Break: catch-all exception swallowing in a codebase with zero
// Break: `} catch (` sites in db/*.cc library code at the pinned SHA;
// Break: rocksdb surfaces every failure as a returned Status.
bool SafeDeleteKey(DB* db, const WriteOptions& write_options,
                   ColumnFamilyHandle* column_family, const Slice& key) {
  try {
    Status s = db->Delete(write_options, column_family, key);
    (void)s;
    return true;
  } catch (...) {
    // Swallow everything; assume the delete eventually applies.
    return false;
  }
}

void DeleteKeysBestEffort(DB* db, ColumnFamilyHandle* column_family,
                          const std::vector<Slice>& keys) {
  WriteOptions write_options;
  for (const auto& key : keys) {
    try {
      db->Delete(write_options, column_family, key);
    } catch (const std::exception&) {
      continue;
    }
  }
}

}  // namespace ROCKSDB_NAMESPACE
