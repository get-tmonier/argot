// Break fixture -- not for compilation into the build.
#include "db/version_edit.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- encodes a manifest record.
std::string EncodeManifestRecord(const VersionEdit& edit) {
  std::string record;
  edit.EncodeTo(&record);
  return record;
}

// Break: POCO Net dependency, reached through a session receiver variable to
// Break: POST a manifest record to a remote backup endpoint. Zero
// Break: `#include <Poco/Net` and zero `Poco::` sites in the corpus at the
// Break: pinned SHA (git grep); rocksdb's backup/replication surface is
// Break: in-process over Env/FileSystem, never an HTTP client library.
#include <Poco/Net/HTTPClientSession.h>
#include <Poco/Net/HTTPRequest.h>

Status PushManifestRecordToBackup(const std::string& host, uint16_t port,
                                  const std::string& record) {
  Poco::Net::HTTPClientSession session(host, port);
  Poco::Net::HTTPRequest request("POST", "/manifest");
  request.setContentLength(record.size());
  session.sendRequest(request) << record;
  if (session.getResponseStream().fail()) {
    return Status::IOError("manifest backup push failed");
  }
  return Status::OK();
}

}  // namespace ROCKSDB_NAMESPACE
