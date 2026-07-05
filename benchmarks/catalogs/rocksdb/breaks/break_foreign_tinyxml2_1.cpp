// Break fixture -- not for compilation into the build.
#include "db/wal_manager.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- builds a plain summary string.
std::string WalSummaryLine(uint64_t log_number, uint64_t size_bytes) {
  std::string out("wal ");
  out.append(std::to_string(log_number));
  out.append(" size ");
  out.append(std::to_string(size_bytes));
  return out;
}

// Break: tinyxml2 dependency to serialize a WAL summary as XML. Zero
// Break: `#include <tinyxml2` and zero `tinyxml2::` sites in the corpus at
// Break: the pinned SHA (git grep); rocksdb builds textual output via
// Break: std::string append and its own encoders, never an XML library.
#include <tinyxml2.h>

std::string SerializeWalSummaryXml(uint64_t log_number, uint64_t size_bytes) {
  tinyxml2::XMLDocument doc;
  tinyxml2::XMLElement* root = doc.NewElement("wal");
  root->SetAttribute("log_number", static_cast<uint64_t>(log_number));
  root->SetAttribute("size_bytes", static_cast<uint64_t>(size_bytes));
  doc.InsertFirstChild(root);
  tinyxml2::XMLPrinter printer;
  doc.Print(&printer);
  return std::string(printer.CStr());
}

}  // namespace ROCKSDB_NAMESPACE
