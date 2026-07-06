// Break fixture -- not for compilation into the build.
#include "db/version_edit.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- every Status is propagated.
Status DecodeEditRecord(VersionEdit* edit, const Slice& record) {
  Status s = edit->DecodeFrom(record);
  if (!s.ok()) {
    return s;
  }
  return Status::OK();
}

// Break: errno-style int return codes and silently dropped Status values.
// Break: rocksdb propagates Status (thousands of `Status` returns in db/)
// Break: and marks intentional drops with PermitUncheckedError; plain -1/0
// Break: codes with errno checks appear nowhere in db/ library code.
#include <cerrno>

int ApplyEditUnchecked(VersionEdit* edit, const Slice& record) {
  errno = 0;
  edit->DecodeFrom(record);
  if (errno != 0) {
    return -1;
  }
  return 0;
}

int ReplayEditList(VersionEdit* edit, const std::vector<Slice>& records) {
  int rc = 0;
  for (const auto& record : records) {
    edit->DecodeFrom(record);
    rc = ApplyEditUnchecked(edit, record);
    if (rc < 0) {
      rc = errno ? -errno : -1;
    }
  }
  return rc;
}

}  // namespace ROCKSDB_NAMESPACE
