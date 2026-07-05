// Break fixture -- not for compilation into the build.
#include "db/version_edit.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- decodes a manifest record.
Status DecodeVersionEditRecord(const Slice& record, VersionEdit* edit) {
  return edit->DecodeFrom(record);
}

// Break: libcurl dependency to fetch a manifest record from a remote URL.
// Break: Zero `#include <curl/` and zero `curl_easy_init` sites in the corpus
// Break: at the pinned SHA (git grep); rocksdb's remote/replication surface is
// Break: in-process (EventListener, WalFilter) over the FileSystem/Env
// Break: abstraction, never an HTTP client.
#include <curl/curl.h>

Status FetchRemoteManifest(const std::string& url, std::string* out) {
  CURL* handle = curl_easy_init();
  if (handle == nullptr) {
    return Status::IOError("failed to init curl handle");
  }
  curl_easy_setopt(handle, CURLOPT_URL, url.c_str());
  curl_easy_setopt(handle, CURLOPT_WRITEDATA, out);
  CURLcode rc = curl_easy_perform(handle);
  curl_easy_cleanup(handle);
  if (rc != CURLE_OK) {
    return Status::IOError("remote manifest fetch failed");
  }
  return Status::OK();
}

}  // namespace ROCKSDB_NAMESPACE
