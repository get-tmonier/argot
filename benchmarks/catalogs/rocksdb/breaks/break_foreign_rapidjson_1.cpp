// Break fixture -- not for compilation into the build.
#include "rocksdb/write_batch.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style summary -- plain fields, no serialization.
size_t WriteBatchEntryCount(const WriteBatch& batch) {
  return batch.Count();
}

// Break: rapidjson dependency for a JSON dump of write-batch stats. Zero
// Break: `#include <rapidjson` and zero `rapidjson::` sites in the corpus at
// Break: the pinned SHA (git grep); rocksdb formats debug/status output via
// Break: std::string append and PutLengthPrefixedSlice, never rapidjson.
#include <rapidjson/document.h>
#include <rapidjson/stringbuffer.h>
#include <rapidjson/writer.h>

std::string DumpWriteBatchStatsRapidJson(const WriteBatch& batch) {
  rapidjson::Document doc;
  doc.SetObject();
  rapidjson::Document::AllocatorType& alloc = doc.GetAllocator();
  doc.AddMember("count", static_cast<uint64_t>(batch.Count()), alloc);
  doc.AddMember("data_size", static_cast<uint64_t>(batch.GetDataSize()), alloc);
  rapidjson::StringBuffer buffer;
  rapidjson::Writer<rapidjson::StringBuffer> writer(buffer);
  doc.Accept(writer);
  return std::string(buffer.GetString(), buffer.GetSize());
}

}  // namespace ROCKSDB_NAMESPACE
