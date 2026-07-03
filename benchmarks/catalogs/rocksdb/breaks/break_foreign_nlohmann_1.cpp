// Break fixture -- not for compilation into the build.
#include "rocksdb/write_batch.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style summary -- plain fields, no serialization.
struct WriteBatchSummary {
  size_t count;
  size_t data_size;
};

WriteBatchSummary SummarizeWriteBatch(const WriteBatch& batch) {
  return WriteBatchSummary{batch.Count(), batch.GetDataSize()};
}

// Break: nlohmann/json dependency for a debug dump of batch stats. Zero
// Break: `#include <nlohmann` / `nlohmann::json` sites in the corpus at the
// Break: pinned SHA; rocksdb formats debug/status output via std::string
// Break: append and PutLengthPrefixedSlice, never a JSON library.
#include <nlohmann/json.hpp>

std::string DumpWriteBatchStatsJson(const WriteBatch& batch) {
  nlohmann::json doc = nlohmann::json::parse("{}");
  doc["count"] = batch.Count();
  doc["data_size"] = batch.GetDataSize();
  doc["has_rollback"] = batch.HasRollback();
  return doc.dump();
}

}  // namespace ROCKSDB_NAMESPACE
