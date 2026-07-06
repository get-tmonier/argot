// Break fixture -- not for compilation into the build.
#include "db/version_edit.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- encodes a manifest record.
std::string EncodeVersionEditRecord(const VersionEdit& edit) {
  std::string record;
  edit.EncodeTo(&record);
  return record;
}

// Break: gRPC dependency to push a manifest record to a remote follower.
// Break: Zero `#include <grpcpp/grpcpp` / `grpc::ServerBuilder` /
// Break: `BuildAndStart` sites in the corpus at the pinned SHA; rocksdb's
// Break: own replication hooks (WalFilter, EventListener::OnFlushCompleted)
// Break: are in-process callbacks, never an RPC transport.
#include <grpcpp/grpcpp.h>

Status StartManifestReplicationServer(const std::string& bind_address,
                                      std::unique_ptr<grpc::Server>* server) {
  grpc::ServerBuilder builder;
  builder.AddListeningPort(bind_address, grpc::InsecureServerCredentials());
  *server = builder.BuildAndStart();
  if (*server == nullptr) {
    return Status::IOError("failed to start manifest replication server");
  }
  return Status::OK();
}

}  // namespace ROCKSDB_NAMESPACE
